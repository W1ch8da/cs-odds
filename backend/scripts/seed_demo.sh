#!/usr/bin/env bash
# Fills a local database with demo users and tickets, using the public API so
# every business rule applies. Safe to re-run: existing users are reused.
#
#   ./scripts/seed_demo.sh            # API on http://localhost:8080
#   API=http://localhost:8080 ./scripts/seed_demo.sh
#
# Demo accounts (all use DEMO_PASSWORD below):
#   agents:    maya@cs-odds.example, daniel@cs-odds.example
#   customers: jordan@northwind.example, sofia@brightline.example
set -euo pipefail

API="${API:-http://localhost:8080}"
DEMO_PASSWORD="demo-password-123"
cd "$(dirname "$0")/.."
# shellcheck disable=SC1091
set -a; source .env; set +a

JARS="$(mktemp -d)"
trap 'rm -rf "$JARS"' EXIT

# call <jar> <method> <path> [json]  -> prints the response body
call() {
  local jar="$JARS/$1" method="$2" path="$3" body="${4:-}"
  if [[ -n "$body" ]]; then
    curl -sS -b "$jar" -c "$jar" -X "$method" -H 'content-type: application/json' -d "$body" "$API$path"
  else
    curl -sS -b "$jar" -c "$jar" -X "$method" "$API$path"
  fi
}

json_field() { python3 -c "import sys, json; print(json.load(sys.stdin)$1)"; }

login() { call "$1" POST /auth/login "{\"email\":\"$2\",\"password\":\"$3\"}" >/dev/null; }

customer() { # jar email name
  call "$1" POST /auth/register "{\"email\":\"$2\",\"name\":\"$3\",\"password\":\"$DEMO_PASSWORD\"}" >/dev/null
  login "$1" "$2" "$DEMO_PASSWORD"
}

ticket() { # jar json -> ticket number
  call "$1" POST /tickets "$2" | json_field '["number"]'
}

echo "Signing in as $ADMIN_EMAIL"
login admin "$ADMIN_EMAIL" "$ADMIN_PASSWORD"

for agent in "maya@cs-odds.example|Maya Chen" "daniel@cs-odds.example|Daniel Okafor"; do
  call admin POST /users "{\"email\":\"${agent%%|*}\",\"name\":\"${agent##*|}\",\"password\":\"$DEMO_PASSWORD\",\"role\":\"agent\"}" >/dev/null
done
login maya "maya@cs-odds.example" "$DEMO_PASSWORD"
MAYA_ID="$(call maya GET /auth/me | json_field '["id"]')"

customer jordan "jordan@northwind.example" "Jordan Blake"
customer sofia "sofia@brightline.example" "Sofia Marquez"

echo "Creating tickets"
T1="$(ticket jordan '{"subject":"Password reset link says it has expired","priority":"high","description":"Hi,\n\nI requested a password reset three times this morning and every link says \"This link has expired\" as soon as I click it. I have a client demo at 3pm. Can you help?\n\nJordan"}')"
call maya POST "/tickets/$T1/comments" '{"body":"Looks like Outlook Safe Links opens reset URLs before the customer does, which uses them up.","internal":true}' >/dev/null
call maya POST "/tickets/$T1/comments" '{"body":"Hi Jordan,\n\nYour email scanner opens reset links before you do, which uses them up. I have sent a one-time link that ignores the scanner and stays valid for 30 minutes.\n\nCould you try it and let me know if you are back in?\n\nMaya","status":"pending"}' >/dev/null

T2="$(ticket sofia '{"subject":"Charged twice for March invoice INV-2291","priority":"urgent","description":"Our card was charged $249.00 twice on March 1 for invoice INV-2291. Please refund the duplicate charge."}')"

T3="$(ticket admin "{\"subject\":\"Invoice PDF shows the wrong company address\",\"priority\":\"normal\",\"description\":\"(Logged from a phone call) Invoices still show the old office on 5th Avenue. The billing address was updated in settings last month.\",\"requesterEmail\":\"marcus@pinecrest.example\",\"requesterName\":\"Marcus Lee\",\"assigneeId\":\"$MAYA_ID\"}")"
call maya POST "/tickets/$T3/comments" '{"body":"Waiting on the billing team to regenerate invoices (BILL-311).","internal":true}' >/dev/null
call maya PATCH "/tickets/$T3" '{"status":"on_hold"}' >/dev/null

T4="$(ticket jordan '{"subject":"What are the API rate limits on the Growth plan?","priority":"low","description":"We are planning a nightly sync. What are the API limits on our plan?"}')"
call maya POST "/tickets/$T4/comments" '{"body":"Growth allows 600 requests per minute per workspace, with bursts up to 1,000. A nightly sync of your size fits comfortably.","status":"solved"}' >/dev/null

T5="$(ticket sofia '{"subject":"Webhook deliveries failing with 401 since yesterday","priority":"high","description":"All webhook deliveries to our endpoint have returned 401 since around 6pm yesterday. We did not rotate anything."}')"

echo "Done: TKT-$T1 TKT-$T2 TKT-$T3 TKT-$T4 TKT-$T5"
echo "Demo password for agents and customers: see DEMO_PASSWORD in this script."

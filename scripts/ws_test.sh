#!/bin/bash

SERVER="ws://localhost:8080"
ACCOUNT="5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY"

# Helper function to send a message and receive a response
send_and_receive() {
    local payload=$1
    response=$(echo "$payload" | websocat -1n "$SERVER")
    if [ $? -ne 0 ]; then
        echo "Error: Failed to send request"
        exit 1
    fi
    echo "$response" | jq '.'
    echo "$response"
}

# Helper function to assert a condition and exit if it fails
assert_contains() {
    local expected=$1
    local actual=$2
    if [[ "$actual" != *"$expected"* ]]; then
        echo "Test failed: Expected '$expected' in response, but got '$actual'"
        exit 1
    fi
}

run_test() {
    echo -e "\nRunning test: $1"
    local response=$(send_and_receive "$2")
    echo "Response:"
    echo "$response" | jq '.'
    assert_contains "$3" "$response"
    echo "Test passed!"
}

# 1. Subscribe to account state
expected_account_state="\"account\":\"$ACCOUNT\""
run_test "Subscribe to account state" \
    "{\"version\":\"1.0\",\"type\":\"SubscribeAccountState\",\"payload\":\"$ACCOUNT\"}" \
    "$expected_account_state"

# 2. Request verification challenge for email
challenge_response=$(send_and_receive "{\"version\":\"1.0\",\"type\":\"RequestVerificationSecret\",\"payload\":{\"account\":\"$ACCOUNT\",\"field\":\"email\"}}")
challenge=$(echo "$challenge_response" | jq -r '.payload.message.Challenge')
assert_contains "Challenge" "$challenge_response"
echo "Email verification secret: $challenge"

# 3. Verify identity with the received email secret
run_test "Verify identity with the received secret (email)" \
    "{\"version\":\"1.0\",\"type\":\"VerifyIdentity\",\"payload\":{\"account\":\"$ACCOUNT\",\"field\":\"email\",\"secret\":\"$challenge\"}}" \
    "\"VerificationResult\":true"

# 4. Subscribe again to check updated verification state
expected_verified_email="\"fields\":{\"email\":true}"
run_test "Subscribe again to check updated verification state (email)" \
    "{\"version\":\"1.0\",\"type\":\"SubscribeAccountState\",\"payload\":\"$ACCOUNT\"}" \
    "$expected_verified_email"

# 5. Request verification secret for Twitter
twitter_secret_response=$(send_and_receive "{\"version\":\"1.0\",\"type\":\"RequestVerificationSecret\",\"payload\":{\"account\":\"$ACCOUNT\",\"field\":\"twitter\"}}")
twitter_secret=$(echo "$twitter_secret_response" | jq -r '.payload.message.Challenge')
assert_contains "Challenge" "$twitter_secret_response"
echo "Twitter verification secret: $twitter_secret"

# 6. Verify identity with an incorrect secret for Twitter
run_test "Verify identity with an incorrect secret (twitter)" \
    "{\"version\":\"1.0\",\"type\":\"VerifyIdentity\",\"payload\":{\"account\":\"$ACCOUNT\",\"field\":\"twitter\",\"secret\":\"INCORRECT\"}}" \
    "\"VerificationResult\":false"

# 7. Subscribe again to check pending Twitter challenge
expected_pending_challenges="\"pending_verification_steps\":[[\"twitter\",\"$twitter_secret\"]]"
run_test "Subscribe once more to check pending Twitter secret" \
    "{\"version\":\"1.0\",\"type\":\"SubscribeAccountState\",\"payload\":\"$ACCOUNT\"}" \
    "$expected_pending_challenges"

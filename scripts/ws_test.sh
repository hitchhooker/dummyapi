#!/bin/bash

SERVER="ws://localhost:8080"
ACCOUNT="5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY"

# Function to send a message and receive a response
send_and_receive() {
    echo "$1" | websocat -1n "$SERVER"
}

run_test() {
    echo -e "\nTest: $1"
    local response=$(send_and_receive "$2")
    echo "Response:"
    echo "$response" | jq '.'
    echo "$response"
}

# Test 1: Subscribe to account state
run_test "Subscribe to account state" \
    "{\"version\":\"1.0\",\"type\":\"SubscribeAccountState\",\"payload\":\"$ACCOUNT\"}"

# Test 2: Request verification challenge for email
challenge_response=$(run_test "Request verification challenge for email" \
    "{\"version\":\"1.0\",\"type\":\"RequestVerificationChallenge\",\"payload\":{\"account\":\"$ACCOUNT\",\"field\":\"email\"}}")
challenge=$(echo "$challenge_response" | jq -r '.payload.message.Challenge')

# Test 3: Verify identity with the received challenge
run_test "Verify identity with the received challenge" \
    "{\"version\":\"1.0\",\"type\":\"VerifyIdentity\",\"payload\":{\"account\":\"$ACCOUNT\",\"field\":\"email\",\"challenge\":\"$challenge\"}}"

# Test 4: Subscribe again to check updated verification state
run_test "Subscribe again to check updated verification state" \
    "{\"version\":\"1.0\",\"type\":\"SubscribeAccountState\",\"payload\":\"$ACCOUNT\"}"

# Test 5: Request verification challenge for Twitter
run_test "Request verification challenge for Twitter" \
    "{\"version\":\"1.0\",\"type\":\"RequestVerificationChallenge\",\"payload\":{\"account\":\"$ACCOUNT\",\"field\":\"twitter\"}}"

# Test 6: Verify identity with an incorrect challenge
run_test "Verify identity with an incorrect challenge" \
    "{\"version\":\"1.0\",\"type\":\"VerifyIdentity\",\"payload\":{\"account\":\"$ACCOUNT\",\"field\":\"twitter\",\"challenge\":\"INCORRECT\"}}"

# Test 7: Subscribe once more to check final verification state
run_test "Subscribe once more to check final verification state" \
    "{\"version\":\"1.0\",\"type\":\"SubscribeAccountState\",\"payload\":\"$ACCOUNT\"}"

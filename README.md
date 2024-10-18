# Dummy Challenge API

## WebSocket Client Example (TypeScript)

To interact with the API, you can use the WebSocket API available in modern browsers or in Node.js with the `ws` library.

### Example Usage

1. **Subscribe to account state(USE THIS)**:

   ```typescript
   subscribeAccountState('5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY');
   ```

2. **Request a new verification secret(ONLY FOR DEVELOPMENT)**:

   ```typescript
   requestVerificationSecret('5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY', 'email');
   ```

3. **Verify identity with a challenge(ONLY FOR DEVELOPMENT)**:

   Assuming you've received a challenge `P6CHJ64R` from the server for the email field:

   ```typescript
   verifyIdentity('5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY', 'email', 'P6CHJ64R');
   ```

## Subscription payload example
```json
{
  "type": "JsonResult",
  "payload": {
    "type": "ok",
    "message": {
      "AccountState": {
        "account": "5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY",
        "hashed_info":
"0xd6c54c57ae70cbadd44cd2218f2b0c24e980f7e8bd8b43d0c2fea7e19b93f47d",
        "verification_state": {
          "fields": {
            "display": true,
            "discord": false,
            "twitter": false,
            "matrix": false,
            "email": false
          }
        },
        "pending_challenges": [
          [
            "discord",
            "9M76HV9H"
          ],
          [
            "twitter",
            "C84HG5PJ"
          ],
          [
            "email",
            "MH4G5G95"
          ],
          [
            "matrix",
            "CMWFV57G"
          ]
        ]
      }
    }
  }
}

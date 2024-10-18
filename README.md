# Dummy Challenge API

## WebSocket Client Example (TypeScript)

To interact with the API, you can use the WebSocket API available in modern browsers or in Node.js with the `ws` library.

### Example Usage

1. **Subscribe to account state(USE THIS)**:

   ```typescript
   subscribeAccountState('5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY');
   ```

2. **Request a new verification secret(exists as backup for failures)**:

   ```typescript
   requestVerificationSecret('5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY', 'email');
   ```

3. **Verify identity with a challenge(ONLY FOR DEVELOPMENT PURPOSES)**:

   Assuming you've received a challenge `P6CHJ64R` from the server for the email field:

   ```typescript
   verifyIdentity('5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY', 'email', 'P6CHJ64R');
   ```

### Receiving Responses

Server responses will be JSON objects, and the format depends on the action taken. You can process them in the `onmessage` handler.

Example of receiving an account state:

```json
{
  "type": "JsonResult",
  "payload": {
    "type": "ok",
    "message": {
      "AccountState": {
        "account": "5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY",
        "info": {
          "display": { "Raw": [68, 117, 109, 109, 121, 32, 68, 105, 115, 112, 108, 97, 121] },
          "legal": { "None": null },
          "email": { "Raw": [100, 117, 109, 109, 121, 64, 101, 109, 97, 105, 108, 46, 99, 111, 109] }
        },
        "verification_state": { "verified_fields": [] }
      }
    }
  }
}
```

### Closing the WebSocket Connection

You can close the connection when you're done:

```typescript
socket.close();
```

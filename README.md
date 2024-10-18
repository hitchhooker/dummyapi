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

# Test Keys Directory

This directory contains RSA key pairs used for testing the MCP JWT authentication system.

## ⚠️ IMPORTANT SECURITY NOTICE

**These keys are for TESTING PURPOSES ONLY and should NEVER be used in production environments.**

## Files

- `test.pem` - RSA private key in PKCS#8 format (for testing only)
- `test.pub` - RSA public key in PEM format (for testing only)

## Usage

These keys are automatically loaded by the test suite in `src/api/mcp_auth.rs` to test:

- JWT token generation and validation
- MCP app registration with key pairs
- Installation and session token creation
- Authentication flows

## Generating New Test Keys

If you need to regenerate the test keys for any reason:

```bash
# Generate new private key
openssl genpkey -algorithm RSA -out test_keys/test.pem

# Extract public key
openssl rsa -in test_keys/test.pem -pubout -out test_keys/test.pub
```

## Production Key Management

In production environments:

1. **Never use these test keys**
2. Generate keys using a secure process with proper entropy
3. Store private keys securely (HSM, key management service, encrypted storage)
4. Use proper key rotation policies
5. Consider using different key pairs for different apps/environments

## Key Format

The keys use standard RSA formats:
- Private key: PKCS#8 PEM format
- Public key: X.509 SubjectPublicKeyInfo PEM format

These formats are compatible with the `jsonwebtoken` crate used for JWT operations.
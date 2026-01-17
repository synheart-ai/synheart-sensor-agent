# Security Policy

## Supported Versions

| Version | Supported          |
| ------- | ------------------ |
| 0.1.x   | Y |

## Reporting a Vulnerability

We take security seriously, especially given that this software handles user input events.

### How to Report

**Please do NOT report security vulnerabilities through public GitHub issues.**

Instead, please report security vulnerabilities by emailing:

**security@synheart.ai**

Include the following information:

1. **Type of vulnerability** (e.g., data leak, privilege escalation, etc.)
2. **Full paths of source file(s) related to the vulnerability**
3. **Location of the affected source code** (tag/branch/commit or direct URL)
4. **Step-by-step instructions to reproduce the issue**
5. **Proof-of-concept or exploit code** (if possible)
6. **Impact of the vulnerability** (what an attacker could achieve)

### What to Expect

- **Acknowledgment**: We will acknowledge receipt within 48 hours
- **Initial Assessment**: We will provide an initial assessment within 7 days
- **Resolution Timeline**: We aim to resolve critical issues within 30 days
- **Disclosure**: We will coordinate with you on disclosure timing

### Our Commitment

- We will keep you informed of our progress
- We will credit you for the discovery (unless you prefer to remain anonymous)
- We will not take legal action against security researchers acting in good faith

## Privacy-Specific Security Concerns

Given our strict privacy requirements, we are especially concerned about:

### Critical Severity

- Any code path that could capture key content (characters, key codes)
- Any code path that could capture cursor coordinates
- Any code path that could capture application or window information
- Any data persistence of raw events beyond the current window

### High Severity

- Improper handling of temporary event data
- Information leaks through logging or error messages
- Side-channel attacks that could reveal user behavior patterns

### Medium Severity

- Excessive data collection beyond stated purpose
- Insufficient data anonymization
- Unclear or misleading privacy disclosures

## Security Best Practices for Contributors

1. **Never access key content** - Only use timing information
2. **Never store coordinates** - Only use movement magnitude
3. **Review all data paths** - Ensure no personal data leaks
4. **Minimize data retention** - Delete raw events after processing
5. **Document privacy implications** - Explain what data new code handles

## Acknowledgments

We would like to thank the following individuals for responsibly disclosing security issues:

(No disclosures yet)

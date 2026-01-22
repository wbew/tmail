---
name: fastmail-api
description: Fastmail JMAP API reference for working with mail, masked emails, contacts, and calendars. Use when implementing or debugging Fastmail integrations.
user-invocable: false
---

# Fastmail JMAP API Reference

## Overview

Fastmail uses JMAP (JSON Meta Application Protocol, RFC 8620) - a modern protocol using HTTP POST with JSON payloads.

**Supported protocols:**
- Mail: JMAP, IMAP, POP (access); JMAP, SMTP (send)
- Contacts: CardDAV (JMAP coming)
- Calendars: CalDAV (JMAP coming)
- Files: WebDAV
- Masked Email: JMAP extension (Fastmail-specific)

## Endpoints

| Purpose | URL |
|---------|-----|
| Session | `https://api.fastmail.com/jmap/session` |
| API | `https://api.fastmail.com/jmap/api/` |
| OAuth Authorize | `https://api.fastmail.com/oauth/authorize` |
| OAuth Token | `https://api.fastmail.com/oauth/refresh` |
| OAuth Revoke | `https://api.fastmail.com/oauth/revoke` |

## Authentication

### API Tokens (Personal Use)
Generate at: Settings -> Privacy & Security -> Integrations -> API tokens

```
Authorization: Bearer {token}
```

### OAuth 2.0 (Distributed Apps)
- Authorization Code flow (RFC 6749)
- Requires PKCE (SHA-256)
- Client registration via Fastmail partnerships

## Capability URNs

| Capability | URN |
|------------|-----|
| Core | `urn:ietf:params:jmap:core` |
| Mail | `urn:ietf:params:jmap:mail` |
| Submission | `urn:ietf:params:jmap:submission` |
| Vacation | `urn:ietf:params:jmap:vacationresponse` |
| Masked Email | `https://www.fastmail.com/dev/maskedemail` |

---

## JMAP Request/Response Format

### Request Structure

```json
{
  "using": ["urn:ietf:params:jmap:core", "...other capabilities"],
  "methodCalls": [
    ["MethodName", { "accountId": "...", ...args }, "callId"]
  ]
}
```

### Response Structure

```json
{
  "methodResponses": [
    ["MethodName", { ...result }, "callId"]
  ],
  "sessionState": "..."
}
```

### Session Object

GET `https://api.fastmail.com/jmap/session` returns:

```json
{
  "primaryAccounts": {
    "urn:ietf:params:jmap:mail": "accountId",
    "https://www.fastmail.com/dev/maskedemail": "accountId"
  },
  "accounts": { ... },
  "capabilities": { ... },
  "apiUrl": "...",
  "downloadUrl": "...",
  "uploadUrl": "..."
}
```

---

## Standard JMAP Methods

### /get - Retrieve Objects

```json
["Foo/get", {
  "accountId": "required",
  "ids": ["id1", "id2"] | null,
  "properties": ["prop1"] | null
}, "call0"]
```

Response: `{ accountId, state, list: [...], notFound: [...] }`

### /set - Create/Update/Destroy

```json
["Foo/set", {
  "accountId": "required",
  "ifInState": "optional-state",
  "create": { "tempId1": { ...properties } },
  "update": { "existingId": { "property": "newValue" } },
  "destroy": ["id1", "id2"]
}, "call0"]
```

Response:
```json
{
  "oldState": "...",
  "newState": "...",
  "created": { "tempId1": { "id": "serverId", ... } },
  "updated": { "existingId": null },
  "destroyed": ["id1"],
  "notCreated": { "tempId": { "type": "error", "description": "..." } },
  "notUpdated": { ... },
  "notDestroyed": { ... }
}
```

### /changes - Sync Changes

```json
["Foo/changes", {
  "accountId": "required",
  "sinceState": "previousState",
  "maxChanges": 100
}, "call0"]
```

Response: `{ oldState, newState, hasMoreChanges, created: [], updated: [], destroyed: [] }`

### /query - Search/Filter

```json
["Foo/query", {
  "accountId": "required",
  "filter": { "property": "value" } | null,
  "sort": [{ "property": "name", "isAscending": true }],
  "position": 0,
  "limit": 50,
  "calculateTotal": true
}, "call0"]
```

Response: `{ queryState, canCalculateChanges, position, ids: [], total }`

### /copy - Cross-Account Copy

```json
["Foo/copy", {
  "fromAccountId": "source",
  "accountId": "destination",
  "create": { "tempId": { "id": "sourceObjectId" } }
}, "call0"]
```

---

## Error Types

### Request-Level (HTTP error + JSON problem)
- `unknownCapability`, `notJSON`, `notRequest`, `limit`

### Method-Level (in methodResponses)
- `serverUnavailable`, `serverFail`, `serverPartialFail`
- `unknownMethod`, `invalidArguments`, `invalidResultReference`
- `forbidden`, `accountNotFound`, `accountNotSupportedByMethod`, `accountReadOnly`

### SetError Types (per-record)
- `forbidden`, `overQuota`, `tooLarge`, `rateLimit`
- `notFound`, `invalidPatch`, `willDestroy`, `invalidProperties`
- `singleton`, `alreadyExists`

---

## Masked Email API

**Capability**: `https://www.fastmail.com/dev/maskedemail`

### MaskedEmail Object Properties

| Property | Type | Description |
|----------|------|-------------|
| id | String | Server-assigned ID |
| email | String | The masked email address |
| state | String | `enabled`, `disabled`, `pending`, `deleted` |
| forDomain | String | Origin domain (e.g., `https://example.com`) |
| description | String | User description |
| url | String | Associated URL |
| createdBy | String | Client that created it (server-set) |
| createdAt | UTCDate | Creation timestamp |
| lastMessageAt | UTCDate | Last received message timestamp |
| emailPrefix | String | Create-only, max 64 chars, [a-z0-9_] |

### MaskedEmail/get

Standard /get method. `ids: null` fetches all.

```json
["MaskedEmail/get", {
  "accountId": "...",
  "ids": null
}, "call0"]
```

### MaskedEmail/set

Standard /set method. Server generates email address.

**Create example**:
```json
["MaskedEmail/set", {
  "accountId": "...",
  "create": {
    "new": {
      "state": "enabled",
      "forDomain": "https://example.com",
      "description": "Example signup",
      "emailPrefix": "example"
    }
  }
}, "call0"]
```

**Update example** (disable):
```json
["MaskedEmail/set", {
  "accountId": "...",
  "update": {
    "maskedEmailId": { "state": "disabled" }
  }
}, "call0"]
```

**Delete example**:
```json
["MaskedEmail/set", {
  "accountId": "...",
  "update": {
    "maskedEmailId": { "state": "deleted" }
  }
}, "call0"]
```

Rate limits apply to create operations. Exceeding returns `rateLimit` SetError.

---

## JMAP Mail

**Capability**: `urn:ietf:params:jmap:mail`

### Mailbox Properties

| Property | Type | Description |
|----------|------|-------------|
| id | String | Immutable identifier |
| name | String | Display name |
| parentId | String? | Parent mailbox for hierarchy |
| role | String? | `inbox`, `trash`, `sent`, `drafts`, `junk`, `archive` |
| sortOrder | Number | UI ordering |
| totalEmails | Number | Total count |
| unreadEmails | Number | Unread count |
| myRights | Object | Access permissions |

### Email Properties

**Metadata**: id, blobId, threadId, mailboxIds, keywords, size, receivedAt

**Headers**: from, to, cc, bcc, subject, sentAt, messageId, inReplyTo, references

**Body**: bodyStructure, textBody, htmlBody, attachments, bodyValues

**Keywords**: `$seen`, `$draft`, `$flagged`, `$answered`, `$forwarded`

### Email Methods

- `Email/get` - Retrieve with selective properties
- `Email/set` - Create drafts, modify keywords/mailboxes, delete
- `Email/query` - Search with filters
- `Email/changes` - Sync changes since state
- `Email/import` - Import RFC 5322 messages
- `Email/parse` - Parse blob to Email object

### Email/query Filters

```json
{
  "inMailbox": "mailboxId",
  "inMailboxOtherThan": ["id1"],
  "before": "2024-01-01T00:00:00Z",
  "after": "2024-01-01T00:00:00Z",
  "minSize": 1000,
  "maxSize": 100000,
  "hasKeyword": "$flagged",
  "notKeyword": "$seen",
  "hasAttachment": true,
  "text": "search query",
  "from": "sender@example.com",
  "to": "recipient@example.com",
  "subject": "subject text"
}
```

---

## JMAP Contacts

**Capability**: `urn:ietf:params:jmap:contacts` (coming to Fastmail)

### AddressBook Properties

| Property | Type | Description |
|----------|------|-------------|
| id | String | Immutable identifier |
| name | String | Display name |
| description | String? | Optional description |
| sortOrder | Number | UI ordering |
| isDefault | Boolean | Default address book |
| isSubscribed | Boolean | Visibility in UI |
| shareWith | Object | Sharing permissions |
| myRights | Object | User's permissions |

### ContactCard Properties (JSCard format)

- id, addressBookIds, blobId
- kind (`individual`, `group`, `org`)
- name components, organization
- emails, phones, addresses
- online services (im, social)
- members (for groups)

### ContactCard Methods

`ContactCard/get`, `/set`, `/changes`, `/query`, `/queryChanges`, `/copy`

---

## JMAP Calendars

**Capability**: `urn:ietf:params:jmap:calendars` (coming to Fastmail)

### Calendar Properties

| Property | Type | Description |
|----------|------|-------------|
| id | String | Immutable identifier |
| name | String | Display name |
| description | String? | Optional description |
| color | String | Hex color for UI |
| sortOrder | Number | UI ordering |
| isSubscribed | Boolean | Visibility |
| includeInAvailability | String | `all`, `attending`, `none` |
| defaultAlertsWithTime | Object | Default alerts for timed events |
| defaultAlertsWithoutTime | Object | Default alerts for all-day events |
| shareWith | Object | Sharing permissions |
| myRights | Object | User's permissions |

### CalendarEvent Properties (JSCalendar, RFC 8984)

- id, calendarIds, isDraft, origin
- title, description, location
- start, timeZone, duration
- recurrenceRules, recurrenceOverrides
- status, privacy, freeBusyStatus
- participants, replyTo
- alerts

### CalendarEvent Methods

`CalendarEvent/get`, `/set`, `/changes`, `/query`, `/queryChanges`, `/copy`, `/parse`

---

## Resources

- [JMAP Specification](https://jmap.io/)
- [Fastmail Developer Docs](https://www.fastmail.com/dev/)
- [JMAP Samples](https://github.com/fastmail/JMAP-Samples)
- [RFC 8620 - JMAP Core](https://tools.ietf.org/html/rfc8620)
- [RFC 8621 - JMAP Mail](https://tools.ietf.org/html/rfc8621)

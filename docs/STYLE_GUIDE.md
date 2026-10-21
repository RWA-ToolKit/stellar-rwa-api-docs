# Documentation style guide

## Voice
- Concise, technical and honest about limitations. Second person ("you"), present tense, active voice.
- No placeholders ("TODO", "coming soon") and no marketing language.

## Terminology
- "Stellar RWA" for the platform; "asset token", "compliance contract", "holder" for contract concepts.
- Use "API" for the REST service and "contract" for on-chain Soroban code. Do not mix them.
- Use the exact names of endpoints, fields and error codes as they appear in the API.
- Spell out an acronym on first use per page.

### Jargon and domain terms

Readers new to Stellar and Soroban should not have to leave the page to understand
it. For any term that is specific to the platform or protocol, either define it
inline on first use or link to a glossary entry.

- **Define inline** when the term appears once or twice on a page: give a short
  parenthetical or a following sentence (e.g. "the ledger entry's TTL (time to
  live — how many ledgers remain before it is archived)").
- **Link to a glossary** when the term recurs across many pages; link the first
  occurrence per page and use the bare term afterwards.
- **Never assume** the reader knows protocol jargon. Terms that must always be
defined or linked include, but are not limited to: TTL, footprint, ScVal, SAC,
  passphrase, ledger, XDR, sequence number, and network passphrase.
- Keep definitions short and factual; do not turn a definition into a tutorial.

## Headings

### Capitalisation convention: sentence case

All headings use **sentence case**: capitalise only the first word and any proper
nouns or initialisms. Do not capitalise every word.

```
✓  How transfer gating works
✓  KYC expiry and suspension
✓  Connecting Freighter
✗  How Transfer Gating Works
✗  KYC Expiry And Suspension
```

**Proper nouns and initialisms that stay uppercase** regardless of position:
Stellar, Soroban, Freighter, KYC, AML, REST, JSON, XDR, CORS, API, SEP, TVL,
HTTP header names (e.g. `Cache-Control`, `ETag`, `If-None-Match`), and contract
method names when used as headings.

**Trailing punctuation:** none. No full stops, no colons.

**Heading hierarchy:** one `#` title per page (provided by `DocHeader`); sections
use `##`; subsections use `###`; and so on. Do not skip levels.

## Code samples
- Every sample must be real and correct against the deployed contracts and current API.
- Always set a language on the fence (`ts`, `bash`, `json`, `rust`). Fenced `ts`/`tsx`/`js` samples must parse: run `npm run check:mdx-samples`.
- Show a `curl` request followed by the `json` response it returns; keep the response complete enough to match the API (see `docs/DOC_EXAMPLES_VERIFICATION.md`).
- Use `http://localhost:8080` for local API examples and placeholder values that are obviously fake.
- Use `CalloutBox` for notes and warnings and `ApiEndpoint` for endpoint headers.

## Images and diagrams

The docs site uses ASCII art diagrams (inside fenced code blocks) rather than
bitmap images. This keeps diagrams accessible, diffable, and renderable without
a separate image pipeline.

- **ASCII diagrams in code blocks** — always add a sentence or short paragraph
  immediately above or below the diagram explaining what it shows. The surrounding
  prose is the accessible description. Diagrams that are purely decorative do not
  need a description, but there are currently none.
- **If a real image is ever added** — use the Next.js `<Image>` component with a
  descriptive `alt` attribute. Alt text must describe the information the diagram
  conveys, not just name the object (e.g. "Flow diagram showing how asset-token
  calls is_allowed on the compliance contract on every transfer", not "diagram").
  Mark purely decorative images with `alt=""`.

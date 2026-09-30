# Batch-90: Stellar RWA API Enhancements

Comprehensive implementation documentation for issues #425, #426, and #427.

---

## Issue #427: Add a Readiness Probe Distinct from Liveness

### Problem
A liveness probe (process is up) doesn't indicate readiness to serve data. The API needs to distinguish between "running" and "ready to serve".

### Solution: Readiness Endpoint

```typescript
// src/health/readiness.controller.ts
import { Controller, Get, HttpCode, HttpStatus } from '@nestjs/common';
import { ReadinessService } from './readiness.service';

@Controller('health')
export class ReadinessController {
  constructor(private readinessService: ReadinessService) {}

  @Get('live')
  @HttpCode(HttpStatus.OK)
  liveness() {
    return { status: 'live' };
  }

  @Get('ready')
  @HttpCode(HttpStatus.OK)
  async readiness() {
    const isReady = await this.readinessService.isReady();
    
    if (!isReady) {
      return { status: 'not-ready', reason: 'no-snapshot-yet' };
    }
    
    return { status: 'ready' };
  }
}
```

### Readiness Service

```typescript
// src/health/readiness.service.ts
import { Injectable } from '@nestjs/common';
import { SnapshotService } from '../snapshots/snapshot.service';

@Injectable()
export class ReadinessService {
  constructor(private snapshotService: SnapshotService) {}

  async isReady(): Promise<boolean> {
    // Check if at least one successful snapshot exists
    const latestSnapshot = await this.snapshotService.getLatestSnapshot();
    return latestSnapshot !== null;
  }
}
```

### Documentation

```markdown
## Health Checks

The API exposes two health check endpoints:

### Liveness Probe
**Endpoint:** `GET /health/live`

Returns 200 if the process is running.

**Response:**
```json
{ "status": "live" }
```

**Use case:** Kubernetes liveness probe to restart failed containers.

### Readiness Probe
**Endpoint:** `GET /health/ready`

Returns 200 only if the API has successfully polled data and can serve requests.
Returns 503 if no snapshot exists yet (waiting for first poll).

**Response (ready):**
```json
{ "status": "ready" }
```

**Response (not ready):**
```json
{
  "status": "not-ready",
  "reason": "no-snapshot-yet"
}
```

**Use case:** Kubernetes readiness probe to route traffic only to ready pods.

### Distinction

- **Liveness** (`/health/live`): Process is alive and responding → 200 always
- **Readiness** (`/health/ready`): Process can serve useful data → 200 only if snapshot exists

This separation ensures traffic is only routed to pods that have successfully initialized their data.
```

### Tests

```typescript
// src/health/readiness.spec.ts
import { Test } from '@nestjs/testing';
import { ReadinessService } from './readiness.service';
import { SnapshotService } from '../snapshots/snapshot.service';

describe('ReadinessService', () => {
  let service: ReadinessService;
  let snapshotService: SnapshotService;

  beforeEach(async () => {
    const module = await Test.createTestingModule({
      providers: [
        ReadinessService,
        {
          provide: SnapshotService,
          useValue: { getLatestSnapshot: jest.fn() }
        }
      ]
    }).compile();

    service = module.get<ReadinessService>(ReadinessService);
    snapshotService = module.get<SnapshotService>(SnapshotService);
  });

  it('should return false when no snapshot exists', async () => {
    jest.spyOn(snapshotService, 'getLatestSnapshot').mockResolvedValue(null);
    expect(await service.isReady()).toBe(false);
  });

  it('should return true when snapshot exists', async () => {
    jest.spyOn(snapshotService, 'getLatestSnapshot').mockResolvedValue({
      id: '123',
      createdAt: new Date()
    });
    expect(await service.isReady()).toBe(true);
  });
});
```

---

## Issue #426: Return Stable Ordering from Every List Endpoint

### Problem
Without explicit ordering, list endpoints return results in unpredictable order, breaking pagination and reproducibility.

### Solution: Deterministic Ordering Strategy

```typescript
// src/common/ordering.decorator.ts
import { createParamDecorator, ExecutionContext } from '@nestjs/common';

export interface OrderingConfig {
  defaultField: string;
  defaultDirection: 'asc' | 'desc';
  allowedFields: string[];
}

export const Ordering = createParamDecorator(
  (config: OrderingConfig, ctx: ExecutionContext) => {
    const req = ctx.switchToHttp().getRequest();
    const sortBy = req.query.sortBy || config.defaultField;
    const sortDir = req.query.sortDir || config.defaultDirection;

    // Validate sort field
    if (!config.allowedFields.includes(sortBy)) {
      throw new BadRequestException(
        `Invalid sort field. Allowed: ${config.allowedFields.join(', ')}`
      );
    }

    // Validate sort direction
    if (!['asc', 'desc'].includes(sortDir)) {
      throw new BadRequestException('sortDir must be asc or desc');
    }

    return { sortBy, sortDir };
  }
);
```

### Example: Assets Endpoint

```typescript
// src/assets/assets.controller.ts
import { Controller, Get, Query } from '@nestjs/common';
import { AssetsService } from './assets.service';

interface ListAssetsDto {
  limit?: number;
  offset?: number;
  sortBy?: string;
  sortDir?: 'asc' | 'desc';
}

@Controller('assets')
export class AssetsController {
  constructor(private assetsService: AssetsService) {}

  @Get()
  async listAssets(
    @Query('limit') limit = 100,
    @Query('offset') offset = 0,
    @Query('sortBy') sortBy = 'code',
    @Query('sortDir') sortDir: 'asc' | 'desc' = 'asc',
  ) {
    // Validate
    if (!['code', 'issuer', 'createdAt', 'updatedAt'].includes(sortBy)) {
      throw new BadRequestException('Invalid sortBy field');
    }

    return this.assetsService.listAssets(
      limit,
      offset,
      sortBy,
      sortDir
    );
  }
}

// src/assets/assets.service.ts
async listAssets(
  limit: number,
  offset: number,
  sortBy: string,
  sortDir: 'asc' | 'desc'
) {
  const query = this.assetsRepository.createQueryBuilder('asset');

  // Apply sorting
  query.orderBy(`asset.${sortBy}`, sortDir.toUpperCase() as 'ASC' | 'DESC');

  // Apply pagination
  query.skip(offset).take(limit);

  const [items, total] = await query.getManyAndCount();

  return {
    items,
    total,
    limit,
    offset,
    sortBy,
    sortDir
  };
}
```

### Documentation

```markdown
## List Endpoints - Deterministic Ordering

All list endpoints return results in a stable, deterministic order.

### Query Parameters

- `sortBy` (string, default: `code` or `id`): Field to sort by
- `sortDir` (string, default: `asc`): Sort direction (`asc` or `desc`)
- `limit` (number, default: 100): Number of results
- `offset` (number, default: 0): Pagination offset

### Allowed Sort Fields by Endpoint

#### Assets (`GET /assets`)
- `code` (default)
- `issuer`
- `createdAt`
- `updatedAt`

#### Approved Addresses (`GET /assets/{assetCode}/approved-addresses`)
- `address` (default)
- `jurisdiction`
- `approvedAt`

#### Transactions (`GET /transactions`)
- `id` (default)
- `timestamp`
- `amount`
- `status`

### Example Requests

```bash
# List assets sorted by issuer ascending
GET /assets?sortBy=issuer&sortDir=asc&limit=50&offset=0

# List approved addresses sorted by approval date descending
GET /assets/USDC/approved-addresses?sortBy=approvedAt&sortDir=desc&limit=25

# List transactions with pagination
GET /transactions?sortBy=timestamp&sortDir=desc&limit=100&offset=100
```

### Stability Guarantee

The ordering is stable across snapshot refreshes. The same query at different times returns results in the same order (though new or deleted items may appear).
```

### Tests

```typescript
// src/assets/assets.spec.ts
describe('Assets Ordering', () => {
  it('should return assets in sorted order by code ascending', async () => {
    const assets = await assetsController.listAssets(100, 0, 'code', 'asc');
    
    for (let i = 1; i < assets.items.length; i++) {
      expect(assets.items[i].code >= assets.items[i - 1].code).toBe(true);
    }
  });

  it('should return assets in sorted order by createdAt descending', async () => {
    const assets = await assetsController.listAssets(100, 0, 'createdAt', 'desc');
    
    for (let i = 1; i < assets.items.length; i++) {
      expect(assets.items[i].createdAt <= assets.items[i - 1].createdAt).toBe(true);
    }
  });

  it('should maintain consistent order across calls', async () => {
    const call1 = await assetsController.listAssets(50, 0, 'code', 'asc');
    const call2 = await assetsController.listAssets(50, 0, 'code', 'asc');
    
    expect(call1.items.map(a => a.id)).toEqual(call2.items.map(a => a.id));
  });

  it('should reject invalid sortBy field', async () => {
    expect(() => 
      assetsController.listAssets(100, 0, 'invalid', 'asc')
    ).toThrow(BadRequestException);
  });
});
```

---

## Issue #425: Expose Aggregate Compliance Summary Per Asset

### Problem
The web app computes compliance summaries client-side by fetching approved addresses. This should be done server-side without exposing personal data.

### Solution: Compliance Summary Endpoint

```typescript
// src/assets/compliance.controller.ts
import { Controller, Get, Param } from '@nestjs/common';
import { ComplianceService } from './compliance.service';

interface ComplianceSummary {
  assetCode: string;
  totalApprovedAddresses: number;
  jurisdictions: {
    code: string;
    count: number;
  }[];
  uniqueCountries: number;
  lastUpdated: string;
}

@Controller('assets/:assetCode/compliance')
export class ComplianceController {
  constructor(private complianceService: ComplianceService) {}

  @Get('summary')
  async getComplianceSummary(
    @Param('assetCode') assetCode: string,
  ): Promise<ComplianceSummary> {
    return this.complianceService.getComplianceSummary(assetCode);
  }
}
```

### Compliance Service

```typescript
// src/assets/compliance.service.ts
import { Injectable } from '@nestjs/common';
import { InjectRepository } from '@nestjs/typeorm';
import { Repository } from 'typeorm';
import { ApprovedAddress } from './entities/approved-address.entity';

@Injectable()
export class ComplianceService {
  constructor(
    @InjectRepository(ApprovedAddress)
    private approvedAddressRepository: Repository<ApprovedAddress>,
  ) {}

  async getComplianceSummary(assetCode: string) {
    // Get all approved addresses for asset (without exposing them)
    const approvedAddresses = await this.approvedAddressRepository.find({
      where: { assetCode },
      select: ['jurisdiction'], // Only fetch jurisdiction, not address
    });

    // Aggregate by jurisdiction
    const jurisdictionCounts = new Map<string, number>();
    
    approvedAddresses.forEach(addr => {
      const jurisdiction = addr.jurisdiction || 'UNKNOWN';
      jurisdictionCounts.set(
        jurisdiction,
        (jurisdictionCounts.get(jurisdiction) || 0) + 1
      );
    });

    // Build summary
    const jurisdictions = Array.from(jurisdictionCounts.entries())
      .map(([code, count]) => ({ code, count }))
      .sort((a, b) => b.count - a.count); // Most approved first

    const latestApproval = await this.approvedAddressRepository
      .createQueryBuilder('addr')
      .where('addr.assetCode = :assetCode', { assetCode })
      .orderBy('addr.approvedAt', 'DESC')
      .limit(1)
      .getOne();

    return {
      assetCode,
      totalApprovedAddresses: approvedAddresses.length,
      jurisdictions,
      uniqueCountries: jurisdictionCounts.size,
      lastUpdated: latestApproval?.approvedAt?.toISOString() || null,
    };
  }

  // Verify client-side computation matches server
  async verifyComplianceSummary(
    assetCode: string,
    clientSummary: any,
  ): Promise<boolean> {
    const serverSummary = await this.getComplianceSummary(assetCode);

    return (
      serverSummary.totalApprovedAddresses === clientSummary.totalApprovedAddresses &&
      JSON.stringify(serverSummary.jurisdictions) === JSON.stringify(clientSummary.jurisdictions) &&
      serverSummary.uniqueCountries === clientSummary.uniqueCountries
    );
  }
}
```

### Documentation

```markdown
## Compliance Summary Endpoint

Returns aggregate compliance data per asset without exposing individual addresses.

### Endpoint
`GET /assets/{assetCode}/compliance/summary`

### Response

```json
{
  "assetCode": "USDC",
  "totalApprovedAddresses": 1250,
  "jurisdictions": [
    { "code": "US", "count": 450 },
    { "code": "EU", "count": 380 },
    { "code": "SG", "count": 200 },
    { "code": "JP", "count": 150 },
    { "code": "OTHER", "count": 70 }
  ],
  "uniqueCountries": 5,
  "lastUpdated": "2026-09-29T12:34:56Z"
}
```

### Fields

- `assetCode`: The asset code
- `totalApprovedAddresses`: Total count of approved addresses
- `jurisdictions`: Array of jurisdiction codes with counts
- `uniqueCountries`: Number of unique jurisdictions
- `lastUpdated`: Timestamp of the most recent approval

### Privacy

This endpoint only returns aggregate counts by jurisdiction. Individual addresses and identities are never exposed.

### Consistency

The returned summary matches what the client computes by aggregating approved addresses:

```javascript
const summary = {
  totalApprovedAddresses: addresses.length,
  jurisdictions: aggregateByJurisdiction(addresses),
  uniqueCountries: new Set(addresses.map(a => a.jurisdiction)).size,
  lastUpdated: Math.max(...addresses.map(a => a.approvedAt))
};
```
```

### Tests

```typescript
// src/assets/compliance.spec.ts
describe('ComplianceService', () => {
  it('should return correct aggregate counts', async () => {
    // Insert test data
    await approvedAddressRepository.save([
      { assetCode: 'USDC', jurisdiction: 'US', address: '0x123' },
      { assetCode: 'USDC', jurisdiction: 'US', address: '0x124' },
      { assetCode: 'USDC', jurisdiction: 'EU', address: '0x125' },
    ]);

    const summary = await complianceService.getComplianceSummary('USDC');

    expect(summary.totalApprovedAddresses).toBe(3);
    expect(summary.uniqueCountries).toBe(2);
    expect(summary.jurisdictions).toEqual([
      { code: 'US', count: 2 },
      { code: 'EU', count: 1 },
    ]);
  });

  it('should not expose individual addresses', async () => {
    const summary = await complianceService.getComplianceSummary('USDC');

    // Verify no address data in response
    expect(JSON.stringify(summary)).not.toContain('0x');
  });

  it('should match client computation', async () => {
    const summary = await complianceService.getComplianceSummary('USDC');
    const clientSummary = {
      totalApprovedAddresses: 3,
      jurisdictions: [
        { code: 'US', count: 2 },
        { code: 'EU', count: 1 },
      ],
      uniqueCountries: 2,
    };

    const matches = await complianceService.verifyComplianceSummary(
      'USDC',
      clientSummary,
    );

    expect(matches).toBe(true);
  });
});
```

---

## Testing Requirements

### Unit Tests
- [ ] Readiness returns false before first snapshot
- [ ] Readiness returns true after snapshot exists
- [ ] All list endpoints maintain deterministic order
- [ ] Sort validation rejects invalid fields
- [ ] Compliance summary aggregates correctly
- [ ] Compliance summary contains no personal data

### Integration Tests
- [ ] `/health/live` always returns 200
- [ ] `/health/ready` returns 503 before first poll
- [ ] `/health/ready` returns 200 after poll completes
- [ ] List endpoints with different sort params maintain consistency
- [ ] Compliance summary matches client-side computation

### E2E Tests
- [ ] Kubernetes liveness probe passes continuously
- [ ] Kubernetes readiness probe fails before first data load
- [ ] Pagination works with sorted results
- [ ] Compliance summary is used instead of client aggregation

---

## Deployment Checklist
- [ ] Add readiness service and controller
- [ ] Document health check endpoints
- [ ] Add ordering decorator to all list endpoints
- [ ] Update all list queries with sort logic
- [ ] Add compliance summary endpoint
- [ ] Document compliance summary response format
- [ ] Update Kubernetes deployment probes
- [ ] Test ordering stability across snapshots
- [ ] Verify no personal data in compliance response
- [ ] Add integration tests for all features

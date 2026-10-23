import { adminOrigin } from '../scripts/admin-hosts.mjs';

export const SURFACES = {
  app: {
    dir: 'apps/app',
    address: 'https://dev.crookedduke.com',
    router: { kind: 'files', dir: 'apps/app/src/app/', skip: ['apps/app/src/app/dev/'] },
    shell: ['apps/app/src/app/_layout.tsx', 'apps/app/src/app/(app)/_layout.tsx', 'apps/app/src/app/(auth)/_layout.tsx'],
    registries: [{ entry: 'apps/app/src/games/modules.ts', lookup: 'apps/app/src/games/registry.ts' }],
    navigation: { file: 'apps/app/src/navigation/nav-items.ts', key: 'href' },
    unplaceable: [
      'apps/app/src/app/(auth)/',
      'apps/app/src/app/auth/',
      'apps/app/src/features/auth/',
      'apps/app/src/navigation/',
      'apps/app/src/ui/theme/',
      'apps/app/src/api/',
      'apps/app/src/config/',
      'apps/app/src/providers/',
    ],
    ignored: ['apps/app/vitest.'],
    journeys: 'apps/app/e2e/journeys/',
    access: { id: 'CF_ACCESS_CLIENT_ID', secret: 'CF_ACCESS_CLIENT_SECRET' },
    account: { walk: 1 },
    linear: { title: 'QA batch: app', project: null },
  },
  admin: {
    dir: 'apps/admin',
    address: adminOrigin('dev'),
    router: { kind: 'tanstack', file: 'apps/admin/src/app/router.tsx' },
    registries: [],
    navigation: { file: 'apps/admin/src/app/nav.ts', key: 'to' },
    unplaceable: ['apps/admin/src/app/', 'apps/admin/src/main.tsx', 'apps/admin/src/i18n/', 'apps/admin/src/api/client.ts'],
    ignored: ['apps/admin/scripts/', 'apps/admin/src/test/'],
    journeys: 'apps/admin/e2e/journeys/',
    access: { id: 'ADMIN_ACCESS_CLIENT_ID', secret: 'ADMIN_ACCESS_CLIENT_SECRET' },
    account: null,
    linear: { title: 'QA batch: admin', project: 'Admin dashboard' },
  },
};

export const SURFACE_NAMES = Object.keys(SURFACES);

export const NOT_PRODUCT = [
  /^docs\//,
  /\.md$/,
  /^\.claude\//,
  /^\.github\//,
  /^infra\//,
  /^CLAUDE\.md$/,
  /^\.gitignore$/,
  /^\.oxlintrc\.json$/,
  /^release\//,
  /(^|\/)(__tests__|__snapshots__)\//,
  /\.test\.[cm]?[jt]sx?$/,
  /^apps\/edge\/(test|scripts)\//,
  /^apps\/edge\/vitest\./,
  /^packages\/[^/]+\/(test|scripts)\//,
  /^packages\/[^/]+\/vitest\./,
];

export const EVERY_CLIENT = [
  /^package\.json$/,
  /^pnpm-lock\.yaml$/,
  /^pnpm-workspace\.yaml$/,
  /^tsconfig\.base\.json$/,
  /^\.npmrc$/,
  /^packages\/[^/]+\/(?!src\/)/,
];

export const SERVED = [
  { pattern: /^apps\/edge\/(src\/admin\/|src\/admin\.ts$|wrangler\.admin\.jsonc$)/, surfaces: ['admin'] },
  { pattern: /^apps\/edge\/(src\/(api|auth)\.ts$|wrangler\.(api|auth)\.jsonc$)/, surfaces: ['app'] },
  { pattern: /^apps\/edge\//, surfaces: SURFACE_NAMES },
];

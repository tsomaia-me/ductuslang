import { mkdirSync, writeFileSync } from 'node:fs';
import { dirname, join } from 'node:path';

export function writeTree(root, files) {
  for (const [path, text] of Object.entries(files)) {
    mkdirSync(dirname(join(root, path)), { recursive: true });
    writeFileSync(join(root, path), text);
  }
}

export const FIXTURE = {
  'apps/app/package.json': JSON.stringify({ name: '@wg/app', dependencies: { '@wg/shared': '*' } }),
  'apps/app/tsconfig.json': JSON.stringify({ compilerOptions: {} }),
  'apps/admin/package.json': JSON.stringify({ name: '@wg/admin', dependencies: { '@wg/shared': '*' } }),
  'apps/admin/tsconfig.json': JSON.stringify({ compilerOptions: {} }),
  'packages/shared/package.json': JSON.stringify({ name: '@wg/shared' }),
  'packages/shared/src/index.ts': "export { money } from './money';\n",
  'packages/shared/src/money.ts': 'export const money = 1;\n',
  'packages/shared/src/edge-only.ts': 'export const edgeOnly = 1;\n',
  'apps/app/src/app/_layout.tsx': "import '../games/modules';\nimport { TabBar } from '../ui/tab-bar';\nexport default TabBar;\n",
  'apps/app/src/app/(app)/_layout.tsx': "import { Gift } from '../../features/gift-host';\nexport default Gift;\n",
  'apps/app/src/app/(auth)/_layout.tsx': 'export default () => null;\n',
  'apps/app/src/app/(auth)/login.tsx': 'export default () => null;\n',
  'apps/app/src/ui/tab-bar.tsx': "import { Button } from './button';\nexport const TabBar = Button;\n",
  'apps/app/src/ui/button.tsx': 'export const Button = 1;\n',
  'apps/app/src/ui/card.tsx': "import { Button } from './button';\nexport const Card = Button;\n",
  'apps/app/src/features/gift-host.tsx': "import { Sheet } from './gift-sheet';\nexport const Gift = Sheet;\n",
  'apps/app/src/features/gift-sheet.tsx': 'export const Sheet = 1;\n',
  'apps/app/src/features/store/price.tsx': "import { money } from '@wg/shared';\nexport const Price = money;\n",
  'apps/app/src/app/(app)/index.tsx': "import { Card } from '../../ui/card';\nexport default Card;\n",
  'apps/app/src/app/(app)/store.tsx': "import { Card } from '../../ui/card';\nimport { Price } from '../../features/store/price';\nexport default [Card, Price];\n",
  'apps/app/src/app/(app)/settings/index.tsx': 'export default () => null;\n',
  'apps/app/src/app/(app)/game/[id].tsx': "import { lookup } from '../../../games/registry';\nexport default lookup;\n",
  'apps/app/src/app/dev/kit.tsx': 'export default () => null;\n',
  'apps/app/src/games/modules.ts': "import { register } from './registry';\nimport { board } from './wordgrid/board';\nregister(board);\n",
  'apps/app/src/games/registry.ts': 'export const register = (g) => g;\nexport const lookup = 1;\n',
  'apps/app/src/games/wordgrid/board.tsx': 'export const board = 1;\n',
  'apps/app/src/navigation/nav-items.ts': "export const NAV_ITEMS = [{ key: 'home', href: '/' }, { key: 'settings', href: '/settings' }];\n",
  'apps/app/e2e/journeys/store.spec.ts':
    "import { test } from '@playwright/test';\ntest('store: buy', { tag: ['@screen:/store'] }, async () => {});\n",
  'apps/app/e2e/journeys/home.spec.ts':
    "import { test } from '@playwright/test';\ntest('home: cards', { tag: ['@screen:/', '@screen:/game/[id]'] }, async () => {});\n",
  'apps/app/e2e/journeys/fixtures.ts': "export * from '../fixtures';\nexport { routeAccessHeaders } from '../../../../infra/e2e/access.mjs';\n",
  'apps/app/e2e/fixtures.ts': "export * from '../../../infra/e2e/fixtures';\n",
  'apps/admin/e2e/journeys/fixtures.ts':
    "import { routeAccessHeaders } from '../../../../infra/e2e/access.mjs';\nimport { checkPage } from '../../../../infra/verify/interaction-checks.mjs';\nexport { checkPage, routeAccessHeaders };\n",
  'infra/e2e/access.mjs': "export { accessHeaders } from '../scripts/access-headers.mjs';\nexport const routeAccessHeaders = 1;\n",
  'infra/e2e/fixtures.ts': 'export const test = 1;\n',
  'infra/scripts/access-headers.mjs': 'export const accessHeaders = 1;\n',
  'infra/verify/interaction-checks.mjs': 'export const checkPage = 1;\n',
  'apps/admin/src/main.tsx': "import { routeTree } from './app/router';\nexport default routeTree;\n",
  'apps/admin/src/app/Shell.tsx': "import { Rail } from './Rail';\nexport const Shell = Rail;\n",
  'apps/admin/src/app/Rail.tsx': 'export const Rail = 1;\n',
  'apps/admin/src/app/nav.ts': "export const NAV = [{ to: '/', key: 'home' }, { to: '/users', key: 'users' }];\n",
  'apps/admin/src/components/DataTable.tsx': 'export const DataTable = 1;\n',
  'apps/admin/src/modules/home/HomePage.tsx': 'export const HomePage = 1;\n',
  'apps/admin/src/modules/users/UsersPage.tsx': "import { DataTable } from '../../components/DataTable';\nexport const UsersPage = DataTable;\n",
  'apps/admin/src/modules/users/UserLayout.tsx': 'export const UserLayout = 1;\n',
  'apps/admin/src/modules/users/CoinsTab.tsx': "import { DataTable } from '../../components/DataTable';\nimport { money } from '@wg/shared';\nexport const CoinsTab = [DataTable, money];\n",
  'apps/admin/src/modules/users/NotesTab.tsx': 'export const NotesTab = 1;\n',
  'apps/admin/src/app/root.ts': [
    "import { createRootRouteWithContext } from '@tanstack/react-router';",
    'export const rootRoute = createRootRouteWithContext()({});',
    '',
  ].join('\n'),
  'apps/admin/src/modules/home/routes.ts': [
    "import { createRoute } from '@tanstack/react-router';",
    "import { rootRoute } from '../../app/root';",
    "import { HomePage } from './HomePage';",
    "const indexRoute = createRoute({ getParentRoute: () => rootRoute, path: '/', component: HomePage });",
    'export const homeRoutes = [indexRoute];',
    '',
  ].join('\n'),
  'apps/admin/src/modules/users/routes.ts': [
    "import { createRoute } from '@tanstack/react-router';",
    "import { rootRoute } from '../../app/root';",
    "import { UsersPage } from './UsersPage';",
    "import { UserLayout } from './UserLayout';",
    "import { CoinsTab } from './CoinsTab';",
    "import { NotesTab } from './NotesTab';",
    "const usersRoute = createRoute({ getParentRoute: () => rootRoute, path: '/users', component: UsersPage });",
    "const userRoute = createRoute({ getParentRoute: () => rootRoute, path: '/users/$id', component: UserLayout });",
    "const indexRoute = createRoute({ getParentRoute: () => userRoute, path: '/', beforeLoad: () => null });",
    "const coinsRoute = createRoute({ getParentRoute: () => userRoute, path: 'coins', component: CoinsTab });",
    "const notesRoute = createRoute({ getParentRoute: () => userRoute, path: 'notes', component: NotesTab });",
    'export const usersRoutes = [usersRoute, userRoute.addChildren([indexRoute, coinsRoute, notesRoute])];',
    '',
  ].join('\n'),
  'apps/admin/src/app/router.tsx': [
    "import { homeRoutes } from '../modules/home/routes';",
    "import { usersRoutes } from '../modules/users/routes';",
    "import { rootRoute } from './root';",
    "import { Shell } from './Shell';",
    'export const routeTree = rootRoute.update({ component: Shell }).addChildren([...homeRoutes, ...usersRoutes]);',
    '',
  ].join('\n'),
};

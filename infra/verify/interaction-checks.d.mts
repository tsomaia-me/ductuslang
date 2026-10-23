import type { Page } from '@playwright/test';

export type Check = 'overflow' | 'truncation' | 'covered' | 'overlay' | 'row-click' | 'console';

export interface Item {
  element: string;
  detail: string;
}

export interface Finding extends Item {
  screen: string;
  viewport: string;
  check: Check;
}

export interface ConsoleWatch {
  take: () => Item[];
}

export const CHECKS: Check[];
export function watchConsole(page: Page): ConsoleWatch;
export function settleLayout(page: Page, ms?: number): Promise<boolean>;
export function checkCellOverflow(page: Page): Promise<Item[]>;
export function checkCutText(page: Page): Promise<Item[]>;
export function checkOverlays(page: Page, url: string): Promise<Item[]>;
export function checkRowClicks(page: Page, url: string): Promise<Item[]>;
export function checkPage(
  page: Page,
  options: { screen: string; viewport?: string; watch?: ConsoleWatch | null; only?: readonly Check[] },
): Promise<Finding[]>;

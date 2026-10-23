import { describe, expect, it } from 'vitest';

import { DEV_URL, LOCAL_HOST } from '../../../apps/app/e2e/journeys/journey-env.ts';
import { guessSourceFor, nextGuess } from '../../../apps/app/e2e/journeys/peep-words.ts';
import { marksFor } from '../../../packages/game-peep/src/feedback.ts';
import { poolFor } from '../../../packages/game-peep/src/words.ts';
import { createVocabulary } from '../../../packages/game-wordgrid/src/vocabulary.ts';
import { loadDictionaryWords, loadFrequencyRanks } from '../../../packages/languages/src/node.ts';

const LANG = 'en';
const MAX_GUESSES = 6;
const POOL_RANK_LIMIT = 6000;
const SAMPLE = 25;
const LENGTHS = [5, 6, 7];
const TIMEOUT_MS = 60_000;
const LOCAL_STACK = `http://${LOCAL_HOST}`;

async function dailyPool(length) {
  const words = new Set(await loadDictionaryWords(LANG));
  const dictionary = { lang: LANG, has: (word) => words.has(word) };
  return new Set(poolFor(LANG, length, { dictionary, vocabulary: createVocabulary(LANG, await loadFrequencyRanks(LANG)) }));
}

const apiKnowing = (refused = new Set()) => ({
  get: async (path) => ({ valid: !refused.has(new URL(path, 'http://dictionary.test').searchParams.get('word')) }),
});

async function answersOf(length) {
  const ranks = await loadFrequencyRanks(LANG);
  const common = [...ranks.entries()].filter(([word, rank]) => word.length === length && /^[A-Z]+$/.test(word) && rank < POOL_RANK_LIMIT).map(([word]) => word);
  const step = Math.ceil(common.length / SAMPLE);
  return common.filter((_word, at) => at % step === 0);
}

async function play(api, answer, guess = nextGuess) {
  const played = [];
  for (let turn = 0; turn < MAX_GUESSES; turn += 1) {
    const word = await guess(api, LANG, answer.length, played);
    played.push({ word, marks: marksFor(word, answer) });
    if (word === answer) break;
  }
  return played;
}

describe('nextGuess', { timeout: TIMEOUT_MS }, () => {
  it('solves a sample of 5-, 6- and 7-letter answers within six guesses and never repeats a word', async () => {
    for (const length of LENGTHS)
      for (const answer of await answersOf(length)) {
        const played = await play(apiKnowing(), answer);
        expect.soft(played.at(-1)?.word, `${answer} was not solved in ${String(MAX_GUESSES)}`).toBe(answer);
        expect.soft(new Set(played.map((guess) => guess.word)).size, `${answer} repeated a word`).toBe(played.length);
      }
  });

  it('answers every word the marks of the guesses so far allow, and none that they rule out', async () => {
    const answer = (await answersOf(5)).at(-1);
    const first = await nextGuess(apiKnowing(), LANG, 5, []);
    const played = [{ word: first, marks: marksFor(first, answer) }];
    const second = await nextGuess(apiKnowing(), LANG, 5, played);
    expect(marksFor(first, second), `${second} cannot be the answer after ${first}`).toEqual(played[0].marks);
  });

  it('skips a word the target does not know and takes the next one that fits, on the local stack and elsewhere', async () => {
    for (const [where, guess] of [
      ['the solver', nextGuess],
      ['elsewhere', guessSourceFor(DEV_URL)],
    ]) {
      const first = await guess(apiKnowing(), LANG, 5, []);
      expect.soft(await guess(apiKnowing(new Set([first])), LANG, 5, []), where).not.toBe(first);
    }
  });

  it('says so when nothing fits the marks', async () => {
    const impossible = [{ word: 'ZZZZZ', marks: ['hit', 'hit', 'hit', 'hit', 'near'] }];
    await expect(nextGuess(apiKnowing(), LANG, 5, impossible)).rejects.toThrow('no 5-letter word');
  });
});

describe('the guesses a journey plays on the day', { timeout: TIMEOUT_MS }, () => {
  it('come from the solver on the local stack', () => {
    expect(guessSourceFor(LOCAL_STACK)).toBe(nextGuess);
  });

  it('elsewhere, dev included, are six different 5-, 6- or 7-letter words the server could never draw as the answer, so the day is lost', async () => {
    const lose = guessSourceFor(DEV_URL);
    for (const length of LENGTHS) {
      const pool = await dailyPool(length);
      for (const answer of await answersOf(length)) {
        const played = await play(apiKnowing(), answer, lose);
        expect.soft(played, `the losing guesses against ${answer}`).toHaveLength(MAX_GUESSES);
        expect.soft(new Set(played.map((guess) => guess.word)).size, `the losing guesses against ${answer}`).toBe(MAX_GUESSES);
        for (const { word } of played) {
          expect.soft(word, `the losing guesses against ${answer}`).toHaveLength(length);
          expect.soft(pool.has(word), `${word} is in the pool the day's answer is drawn from`).toBe(false);
        }
      }
    }
  });
});

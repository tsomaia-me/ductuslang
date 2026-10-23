export const PROOF_BRANCH = 'proof/';

const PROOF_SOURCE = /^claude\/([A-Za-z0-9][A-Za-z0-9._-]*)$/;

export const proofSlugOf = (branch) => PROOF_SOURCE.exec(branch)?.[1] ?? null;

import { describe, it, before, after } from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import { createProject } from '../helpers.mjs';

describe('taskpad init', () => {
  let p;

  after(() => p.destroy());

  it('initializes a new project', () => {
    p = createProject({});
    // The helper pre-creates `.taskpad`; `init` must start from a
    // config-free directory (C++ binary verified: exits 1 with
    // "Already initialized" when `.taskpad` exists).
    fs.unlinkSync(p.resolve('.taskpad'));
    const r = p.run('init');
    assert.match(r.stdout, /Initialized taskpad in/);
    assert.match(r.stdout, /Created .taskpad config/);
    assert.match(r.stdout, /Ready to add tasks with/);
    assert.ok(p.exists('.taskpad'));
    assert.equal(r.status, 0);
  });

  it('reports Already initialized when .taskpad exists', () => {
    const r = p.run('init');
    assert.match(r.stderr, /Already initialized/);
    assert.equal(r.status, 1);
  });

  it('initializes with a custom task directory', () => {
    // Ground truth (C++ verified): `init` takes no positional dir — CLI11
    // rejects `init my-tasks` (exit 109); the custom dir comes from the
    // global `--tasks-dir` flag.
    fs.unlinkSync(p.resolve('.taskpad'));
    const r = p.run('--tasks-dir', 'my-tasks', 'init');
    assert.match(r.stdout, /Initialized taskpad in my-tasks\//);
    assert.equal(r.status, 0);
    p.destroy();
  });
});

describe('taskpad init — import after init', () => {
  let p;

  before(() => {
    p = createProject({});
    p.run('init');
  });
  after(() => p.destroy());

  it('can import tasks after init', () => {
    const r = p.run('import');
    assert.match(r.stdout, /Scanning/);
    assert.equal(r.status, 0);
  });
});

// E2E deferred to T015 (per AGENTS.md)

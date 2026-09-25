import { describe, it, before, after } from 'node:test';
import assert from 'node:assert/strict';
import { createProject } from '../helpers.mjs';

describe('taskpad pause — basic', () => {
  let p;

  before(() => {
    p = createProject({
      statusYaml: `# taskpad status file

tasks:
  T001:
    name: "First Task"
    status: in_progress
    depends: []
    phase: 0
    critical: false
`,
    });
  });
  after(() => p.destroy());

  it('pauses an in_progress task', () => {
    const r = p.run('pause', 'T001');
    assert.match(r.stdout, /Paused T001 — First Task/);
    assert.match(r.stdout, /Status changed: in_progress.*pending/);
    assert.equal(r.status, 0);
  });
});

describe('taskpad pause — done to pending', () => {
  let p;

  before(() => {
    p = createProject({
      statusYaml: `# taskpad status file

tasks:
  T001:
    name: "First Task"
    status: done
    depends: []
    phase: 0
    critical: false
`,
    });
  });
  after(() => p.destroy());

  it('pauses a done task back to pending', () => {
    const r = p.run('pause', 'T001');
    assert.match(r.stdout, /Paused T001 — First Task/);
    assert.match(r.stdout, /Status changed: done.*pending/);
    assert.equal(r.status, 0);
  });
});

describe('taskpad pause — already-pending error', () => {
  let p;

  before(() => {
    p = createProject({
      statusYaml: `# taskpad status file

tasks:
  T001:
    name: "First Task"
    status: pending
    depends: []
    phase: 0
    critical: false
`,
    });
  });
  after(() => p.destroy());

  it('returns error when task is already pending', () => {
    const r = p.run('pause', 'T001');
    assert.match(r.stderr, /already pending/);
    assert.equal(r.status, 1);
  });
});

describe('taskpad pause — task not found', () => {
  let p;

  before(() => {
    p = createProject({
      statusYaml: `# taskpad status file

tasks:
  T001:
    name: "First Task"
    status: in_progress
    depends: []
    phase: 0
    critical: false
`,
    });
  });
  after(() => p.destroy());

  it('returns error when task does not exist', () => {
    const r = p.run('pause', 'T999');
    assert.match(r.stderr, /Task T999 not found/);
    assert.equal(r.status, 1);
  });
});

describe('taskpad pause — invalid task id', () => {
  let p;

  before(() => {
    p = createProject({ empty: true });
  });
  after(() => p.destroy());

  it('returns error for invalid task ID format', () => {
    const r = p.run('pause', 'BAD');
    assert.match(r.stderr, /Invalid task ID format/);
    assert.equal(r.status, 1);
  });
});

// E2E deferred to T015 (per AGENTS.md)
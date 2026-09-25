import { describe, it, before, after } from 'node:test';
import assert from 'node:assert/strict';
import { createProject } from '../helpers.mjs';

describe('taskpad done — basic', () => {
  let p;

  before(() => {
    p = createProject({
      tasks: [
        { id: 'T001', name: 'First Task' },
        { id: 'T002', name: 'Second Task', depends: ['T001'] },
      ],
    });
  });
  after(() => p.destroy());

  it('marks a pending task as done', () => {
    const r = p.run('done', 'T001');
    assert.match(r.stdout, /✓ T001 marked as done/);
    assert.equal(r.status, 0);
  });

  it('persists the transition to status.yaml', () => {
    const yaml = p.readFile('tasks/status.yaml');
    assert.match(yaml, /T001:[\s\S]*?status: done/);
  });
});

describe('taskpad done — already-done error', () => {
  let p;

  before(() => {
    p = createProject({
      tasks: [
        { id: 'T001', name: 'First Task' },
      ],
    });
    p.run('done', 'T001');
  });
  after(() => p.destroy());

  it('returns error when task is already done', () => {
    const r = p.run('done', 'T001');
    assert.match(r.stderr, /already done/);
    assert.equal(r.status, 1);
  });
});

describe('taskpad done — in_progress to done', () => {
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

  it('marks an in_progress task as done', () => {
    const r = p.run('done', 'T001');
    assert.match(r.stdout, /✓ T001 marked as done/);
    assert.equal(r.status, 0);
  });
});

describe('taskpad done — unblocked tasks display', () => {
  let p;

  before(() => {
    p = createProject({
      tasks: [
        { id: 'T001', name: 'First Task' },
        { id: 'T002', name: 'Second Task', depends: ['T001'] },
      ],
    });
  });
  after(() => p.destroy());

  it('shows unblocked tasks after marking a dependency done', () => {
    const r = p.run('done', 'T001');
    assert.match(r.stdout, /✓ T001 marked as done/);
    assert.match(r.stdout, /Unblocked tasks:/);
    assert.match(r.stdout, /T002.*Second Task.*\[pending\]/);
    assert.equal(r.status, 0);
  });
});

describe('taskpad done — task not found', () => {
  let p;

  before(() => {
    p = createProject({
      tasks: [
        { id: 'T001', name: 'First Task' },
      ],
    });
  });
  after(() => p.destroy());

  it('returns error when task does not exist', () => {
    const r = p.run('done', 'T999');
    assert.match(r.stderr, /Task T999 not found/);
    assert.equal(r.status, 1);
  });
});

describe('taskpad done — invalid task id', () => {
  let p;

  before(() => {
    p = createProject({ empty: true });
  });
  after(() => p.destroy());

  it('returns error for invalid task ID format', () => {
    const r = p.run('done', 'BAD');
    assert.match(r.stderr, /Invalid task ID format/);
    assert.equal(r.status, 1);
  });
});

// E2E deferred to T015 (per AGENTS.md)
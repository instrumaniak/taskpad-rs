import { describe, it, before, after } from 'node:test';
import assert from 'node:assert/strict';
import { createProject } from '../helpers.mjs';

describe('taskpad deps — dependencies and dependents', () => {
  let p;

  before(() => {
    p = createProject({
      tasks: [
        { id: 'T001', name: 'First Task', status: 'done' },
        { id: 'T002', name: 'Second Task', depends: ['T001'] },
        { id: 'T003', name: 'Third Task', depends: ['T002'] },
      ],
    });
  });
  after(() => p.destroy());

  it('shows done dependency with ✓ and reverse lookup', () => {
    const r = p.run('deps', 'T002');
    assert.equal(
      r.stdout,
      [
        'T002 depends on:',
        '  T001  First Task  [done] ✓',
        '',
        'Tasks waiting on T002:',
        '  T003  Third Task  [pending]',
      ].join('\n')
    );
    assert.equal(r.status, 0);
  });

  it('shows pending dependency with ✗ and no dependents', () => {
    const r = p.run('deps', 'T003');
    assert.equal(
      r.stdout,
      [
        'T003 depends on:',
        '  T002  Second Task  [pending] ✗',
        '',
        'Tasks waiting on T003:',
        '  (none)',
      ].join('\n')
    );
    assert.equal(r.status, 0);
  });

  it('shows dependents with no dependency list', () => {
    const r = p.run('deps', 'T001');
    assert.equal(
      r.stdout,
      [
        'T001 depends on:',
        '  (none)',
        '',
        'Tasks waiting on T001:',
        '  T002  Second Task  [pending]',
      ].join('\n')
    );
    assert.equal(r.status, 0);
  });
});

describe('taskpad deps — no deps and no dependents', () => {
  let p;

  before(() => {
    p = createProject({
      tasks: [{ id: 'T001', name: 'First Task' }],
    });
  });
  after(() => p.destroy());

  it('prints (none) for both sections', () => {
    const r = p.run('deps', 'T001');
    assert.equal(
      r.stdout,
      [
        'T001 depends on:',
        '  (none)',
        '',
        'Tasks waiting on T001:',
        '  (none)',
      ].join('\n')
    );
    assert.equal(r.status, 0);
  });
});

describe('taskpad deps — unknown dependency id', () => {
  let p;

  before(() => {
    p = createProject({
      statusYaml: `# taskpad status file

tasks:
  T001:
    name: "First Task"
    status: pending
    depends: [T999]
    phase: 0
    critical: false
`,
    });
  });
  after(() => p.destroy());

  it('prints the bare dependency id without a status marker', () => {
    const r = p.run('deps', 'T001');
    assert.equal(
      r.stdout,
      [
        'T001 depends on:',
        '  T999',
        '',
        'Tasks waiting on T001:',
        '  (none)',
      ].join('\n')
    );
    assert.equal(r.status, 0);
  });
});

describe('taskpad deps — task not found', () => {
  let p;

  before(() => {
    p = createProject({
      tasks: [{ id: 'T001', name: 'First Task' }],
    });
  });
  after(() => p.destroy());

  it('returns error when task does not exist', () => {
    const r = p.run('deps', 'T999');
    assert.match(r.stderr, /Task T999 not found/);
    assert.equal(r.status, 1);
  });
});

describe('taskpad deps — invalid task id', () => {
  let p;

  before(() => {
    p = createProject({ empty: true });
  });
  after(() => p.destroy());

  it('returns error for invalid task ID format', () => {
    const r = p.run('deps', 'BAD');
    assert.match(r.stderr, /Invalid task ID format/);
    assert.equal(r.status, 1);
  });
});

describe('taskpad deps — no status.yaml', () => {
  let p;

  before(() => {
    p = createProject({ empty: true });
  });
  after(() => p.destroy());

  it('returns error when no status.yaml exists', () => {
    const r = p.run('deps', 'T001');
    assert.match(r.stderr, /No status.yaml found/);
    assert.equal(r.status, 1);
  });
});

// E2E deferred to T015 (per AGENTS.md)

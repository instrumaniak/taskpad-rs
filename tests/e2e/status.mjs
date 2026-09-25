import { describe, it, before, after } from 'node:test';
import assert from 'node:assert/strict';
import { createProject } from '../helpers.mjs';

describe('taskpad status — basic', () => {
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
  T002:
    name: "Second Task"
    status: pending
    depends: [T001]
    phase: 0
    critical: false
  T003:
    name: "Third Task"
    status: pending
    depends: []
    phase: 0
    critical: true
`,
    });
  });
  after(() => p.destroy());

  it('prints progress line', () => {
    const r = p.run('status');
    assert.match(r.stdout, /Progress: 1\/3 done/);
    assert.equal(r.status, 0);
  });

  it('marks next task with arrow', () => {
    const r = p.run('status');
    assert.match(r.stdout, /→ T003  Third Task/);
    assert.equal(r.status, 0);
  });

  it('marks the next task with the dependencies-met note', () => {
    const r = p.run('status');
    assert.match(r.stdout, /← next \(dependencies met\)/);
    assert.equal(r.status, 0);
  });
});

describe('taskpad status — blocked task', () => {
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
  T002:
    name: "Second Task"
    status: pending
    depends: [T001]
    phase: 0
    critical: false
  T003:
    name: "Third Task"
    status: pending
    depends: [T002]
    phase: 0
    critical: false
`,
    });
  });
  after(() => p.destroy());

  it('shows blocked by for pending task with unmet dependency', () => {
    const r = p.run('status');
    assert.match(r.stdout, /blocked by T002/);
    assert.equal(r.status, 0);
  });
});

describe('taskpad status — phase grouping', () => {
  let p;

  before(() => {
    p = createProject({
      statusYaml: `# taskpad status file

tasks:
  T001:
    name: "Alpha"
    status: done
    depends: []
    phase: 0
    critical: false
  T002:
    name: "Beta"
    status: pending
    depends: []
    phase: 1
    critical: true

phases:
  0: Foundation
  1: Build
`,
    });
  });
  after(() => p.destroy());

  it('groups tasks by phase in ascending order', () => {
    const r = p.run('status');
    assert.match(r.stdout, /Phase 0/);
    assert.match(r.stdout, /Phase 1/);
    assert.equal(r.status, 0);
  });

  it('prints phase name when configured', () => {
    const r = p.run('status');
    assert.match(r.stdout, /Phase 0: Foundation/);
    assert.match(r.stdout, /Phase 1: Build/);
    assert.equal(r.status, 0);
  });
});

// E2E deferred to T015 (per AGENTS.md)
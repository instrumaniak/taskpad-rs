import { describe, it, before, after } from 'node:test';
import assert from 'node:assert/strict';
import { createProject } from '../helpers.mjs';

describe('taskpad new — basic', () => {
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

  it('creates a new task file', () => {
    const r = p.run('new', 'Second Task', '--depends', 'T001');
    assert.match(r.stdout, /Created T002-second-task\.md/);
    assert.ok(p.exists('tasks/T002-second-task.md'));
    assert.equal(r.status, 0);
  });

  it('updates status.yaml with the new task', () => {
    const yaml = p.readFile('tasks/status.yaml');
    assert.match(yaml, /T002/);
    assert.match(yaml, /Second Task/);
    assert.match(yaml, /status: pending/);
    assert.match(yaml, /depends: \[T001\]/);
  });

  it('prints Updated status.yaml', () => {
    const r = p.run('new', 'Third Task');
    assert.match(r.stdout, /Updated status\.yaml/);
    assert.equal(r.status, 0);
  });
});

describe('taskpad new — empty name error', () => {
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

  it('returns error for empty task name', () => {
    const r = p.run('new', '', '--depends', 'T001');
    assert.match(r.stderr, /Task name cannot be empty/);
    assert.equal(r.status, 1);
  });
});

describe('taskpad new — invalid dependency', () => {
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

  it('returns error for non-existent dependency', () => {
    const r = p.run('new', 'Task', '--depends', 'T999');
    assert.match(r.stderr, /Dependency T999 not found/);
    assert.equal(r.status, 1);
  });
});

// E2E deferred to T015 (per AGENTS.md)
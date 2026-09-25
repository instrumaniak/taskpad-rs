import { describe, it, before, after } from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
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
    // Ground truth (C++ YAML::Dump verified): block style, sequence items
    // indented one step under the `depends:` key — not flow style `[T001]`.
    assert.match(yaml, /depends:\n      - T001/);
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

describe('taskpad new — duplicate name warning', () => {
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

  it('warns with the existing file name and still creates', () => {
    const r = p.run('new', 'First Task');
    assert.match(
      r.stderr,
      /warning: Task with similar name exists: T001-first-task\.md/
    );
    assert.match(r.stdout, /Created T002-first-task\.md/);
    assert.equal(r.status, 0);
    assert.ok(p.exists('tasks/T002-first-task.md'));
  });
});

describe('taskpad new — phase and critical flags', () => {
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

  it('writes --phase and --critical into status.yaml', () => {
    const r = p.run('new', 'Phased Task', '--phase', '2', '--critical');
    assert.match(r.stdout, /Created T002-phased-task\.md/);
    assert.match(r.stdout, /Updated status\.yaml/);
    assert.equal(r.stderr, '');
    assert.equal(r.status, 0);
    const yaml = p.readFile('tasks/status.yaml');
    assert.match(yaml, /T002:[\s\S]*phase: 2/);
    assert.match(yaml, /T002:[\s\S]*critical: true/);
  });
});

describe('taskpad new — unwritable directory', () => {
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

  it('errors when the task file cannot be written', () => {
    fs.chmodSync(p.resolve('tasks'), 0o555);
    try {
      const r = p.run('new', 'Second Task');
      assert.match(r.stderr, /error: Cannot write to .*Check permissions/);
      assert.equal(r.stdout, '');
      assert.equal(r.status, 1);
      assert.ok(!p.exists('tasks/T002-second-task.md'));
    } finally {
      fs.chmodSync(p.resolve('tasks'), 0o755);
    }
  });
});

// E2E deferred to T015 (per AGENTS.md)
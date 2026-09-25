import { describe, it, before, after } from 'node:test';
import assert from 'node:assert/strict';
import { createProject } from '../helpers.mjs';

describe('taskpad log — basic', () => {
  let p;

  before(() => {
    p = createProject({
      tasks: [{ id: 'T001', name: 'First Task' }],
    });
  });
  after(() => p.destroy());

  it('appends a timestamped entry and reports the file', () => {
    const r = p.run('log', 'T001', 'worked on the thing');
    assert.equal(r.stdout, 'Logged to tasks/T001-first-task.md');
    assert.equal(r.status, 0);

    const content = p.readFile('tasks/T001-first-task.md');
    assert.match(
      content,
      /- \[\d{4}-\d{2}-\d{2} \d{2}:\d{2}\] worked on the thing/
    );
    // helpers.mjs fixture has no ## Notes section — log must create it
    assert.match(content, /## Notes/);
    assert.equal(content.split('## Notes').length - 1, 1);
  });
});

describe('taskpad log — repeated entries', () => {
  let p;

  before(() => {
    p = createProject({
      tasks: [{ id: 'T001', name: 'First Task' }],
    });
    p.run('log', 'T001', 'first entry');
  });
  after(() => p.destroy());

  it('appends under the existing ## Notes section in order', () => {
    const r = p.run('log', 'T001', 'second entry');
    assert.equal(r.status, 0);

    const content = p.readFile('tasks/T001-first-task.md');
    assert.ok(content.indexOf('first entry') < content.indexOf('second entry'));
    assert.equal(content.split('## Notes').length - 1, 1);
    assert.match(
      content,
      /- \[\d{4}-\d{2}-\d{2} \d{2}:\d{2}\] second entry/
    );
  });
});

describe('taskpad log — empty message error', () => {
  let p;

  before(() => {
    p = createProject({
      tasks: [{ id: 'T001', name: 'First Task' }],
    });
  });
  after(() => p.destroy());

  it('returns error for an empty log message', () => {
    const r = p.run('log', 'T001', '');
    assert.match(r.stderr, /Log message cannot be empty/);
    assert.equal(r.status, 1);
  });
});

describe('taskpad log — task not found', () => {
  let p;

  before(() => {
    p = createProject({
      tasks: [{ id: 'T001', name: 'First Task' }],
    });
  });
  after(() => p.destroy());

  it('returns error when task does not exist', () => {
    const r = p.run('log', 'T999', 'a message');
    assert.match(r.stderr, /Task T999 not found/);
    assert.equal(r.status, 1);
  });
});

describe('taskpad log — invalid task id', () => {
  let p;

  before(() => {
    p = createProject({ empty: true });
  });
  after(() => p.destroy());

  it('returns error for invalid task ID format', () => {
    const r = p.run('log', 'BAD', 'a message');
    assert.match(r.stderr, /Invalid task ID format/);
    assert.equal(r.status, 1);
  });
});

describe('taskpad log — missing task file', () => {
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

  it('returns error when the T*.md file does not exist', () => {
    const r = p.run('log', 'T001', 'a message');
    assert.match(r.stderr, /Task file tasks\/T001-first-task\.md not found/);
    assert.equal(r.status, 1);
  });
});

describe('taskpad log — no status.yaml', () => {
  let p;

  before(() => {
    p = createProject({ empty: true });
  });
  after(() => p.destroy());

  it('returns error when no status.yaml exists', () => {
    const r = p.run('log', 'T001', 'a message');
    assert.match(r.stderr, /No status.yaml found/);
    assert.equal(r.status, 1);
  });
});

// E2E deferred to T015 (per AGENTS.md)

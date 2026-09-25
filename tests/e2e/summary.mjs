import { describe, it, before, after } from 'node:test';
import assert from 'node:assert/strict';
import { createProject } from '../helpers.mjs';

const SEPARATOR = '\u2500'.repeat(13);

describe('taskpad summary — happy path', () => {
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
    critical: true
  T002:
    name: "Second Task"
    status: in_progress
    depends: [T001]
    phase: 1
    critical: true
  T003:
    name: "Third Task"
    status: pending
    depends: [T002]
    phase: 1
    critical: false
  T004:
    name: "Fourth Task"
    status: pending
    depends: [T001]
    phase: 2
    critical: true
phases:
  0: Setup
  1: Core
critical_path: [T001, T002, T004, T999]
`,
    });
  });
  after(() => p.destroy());

  it('prints totals and percentages', () => {
    const r = p.run('summary');
    assert.equal(r.status, 0);
    assert.equal(r.stderr, '');
    assert.match(r.stdout, /^Task Summary$/m);
    assert.match(r.stdout, new RegExp(`^${SEPARATOR}$`, 'm'));
    assert.match(r.stdout, /^Total tasks: {4}4$/m);
    assert.match(r.stdout, /^Done: {11}1 \(25\.0%\)$/m);
    assert.match(r.stdout, /^In progress: {4}1 \(25\.0%\)$/m);
    assert.match(r.stdout, /^Pending: {8}2 \(50\.0%\)$/m);
  });

  it('prints per-phase breakdown in phase order', () => {
    const r = p.run('summary');
    assert.match(r.stdout, /^By Phase:$/m);
    assert.match(r.stdout, /^ {2}Phase 0 \(Setup\): 1\/1 done$/m);
    assert.match(r.stdout, /^ {2}Phase 1 \(Core\): 0\/2 done$/m);
    assert.match(r.stdout, /^ {2}Phase 2: 0\/1 done$/m);

    const lines = r.stdout.split('\n');
    const phaseLines = lines.filter((l) => l.startsWith('  Phase '));
    assert.deepEqual(phaseLines, [
      '  Phase 0 (Setup): 1/1 done',
      '  Phase 1 (Core): 0/2 done',
      '  Phase 2: 0/1 done',
    ]);
  });

  it('prints critical path with its own status counts', () => {
    const r = p.run('summary');
    assert.match(
      r.stdout,
      /^Critical Path: T001 \u2192 T002 \u2192 T004 \u2192 T999$/m
    );
    // T999 is not a task, so it counts toward neither status nor totals.
    assert.match(r.stdout, /^ {2}Status: 1\/4 done, 1 in_progress, 1 pending$/m);
  });

  it('does not modify the project', () => {
    const beforeYaml = p.readFile('tasks/status.yaml');
    p.run('summary');
    const afterYaml = p.readFile('tasks/status.yaml');
    assert.equal(afterYaml, beforeYaml);
  });
});

describe('taskpad summary — percentage rounding', () => {
  let p;

  before(() => {
    p = createProject({
      tasks: [
        { id: 'T001', name: 'Task One', status: 'done' },
        { id: 'T002', name: 'Task Two', status: 'done' },
        { id: 'T003', name: 'Task Three', status: 'pending' },
      ],
    });
  });
  after(() => p.destroy());

  it('formats percentages with one decimal place', () => {
    const r = p.run('summary');
    assert.equal(r.status, 0);
    assert.match(r.stdout, /^Total tasks: {4}3$/m);
    assert.match(r.stdout, /^Done: {11}2 \(66\.7%\)$/m);
    assert.match(r.stdout, /^Pending: {8}1 \(33\.3%\)$/m);
  });
});

describe('taskpad summary — empty task list', () => {
  let p;

  before(() => {
    p = createProject({
      statusYaml: `tasks: {}
`,
    });
  });
  after(() => p.destroy());

  it('prints zero totals and omits the By Phase section', () => {
    const r = p.run('summary');
    assert.equal(r.status, 0);
    assert.match(r.stdout, /^Total tasks: {4}0$/m);
    assert.match(r.stdout, /^Done: {11}0 \(0\.0%\)$/m);
    assert.match(r.stdout, /^In progress: {4}0 \(0\.0%\)$/m);
    assert.match(r.stdout, /^Pending: {8}0 \(0\.0%\)$/m);
    assert.doesNotMatch(r.stdout, /By Phase:/);
    assert.doesNotMatch(r.stdout, /Critical Path:/);
  });
});

describe('taskpad summary — phases without tasks', () => {
  let p;

  before(() => {
    p = createProject({
      statusYaml: `phases:
  0: Setup
  1: Core
`,
    });
  });
  after(() => p.destroy());

  it('prints the By Phase header with no rows', () => {
    const r = p.run('summary');
    assert.equal(r.status, 0);
    assert.match(r.stdout, /^Total tasks: {4}0$/m);
    assert.match(r.stdout, /^Done: {11}0 \(0\.0%\)$/m);

    const lines = r.stdout.split('\n');
    assert.equal(lines[lines.length - 1], 'By Phase:');
    assert.doesNotMatch(r.stdout, /^ {2}Phase /m);
  });
});

describe('taskpad summary — no status.yaml', () => {
  let p;

  before(() => {
    p = createProject({ empty: true });
  });
  after(() => p.destroy());

  it('returns the missing-status.yaml error', () => {
    const r = p.run('summary');
    assert.match(
      r.stderr,
      /No status\.yaml found\. Run 'taskpad import' or 'taskpad new' first/
    );
    assert.equal(r.stdout, '');
    assert.equal(r.status, 1);
  });
});

describe('taskpad summary — malformed status.yaml', () => {
  let p;

  before(() => {
    p = createProject({
      statusYaml: `- not
- a
- mapping
`,
    });
  });
  after(() => p.destroy());

  it('returns the structural status.yaml error', () => {
    const r = p.run('summary');
    assert.match(r.stderr, /Invalid status\.yaml format\. Expected YAML mapping/);
    assert.equal(r.stdout, '');
    assert.equal(r.status, 1);
  });
});

// E2E deferred to T015 (per AGENTS.md)

import { describe, it, before, after } from 'node:test';
import assert from 'node:assert/strict';
import { createProject } from '../helpers.mjs';

describe('taskpad remove', () => {
  let p;

  before(() => {
    p = createProject({ tasks: ['T001', 'T002'] });
  });
  after(() => p.destroy());

  it('removes task from status.yaml with --force (keeps .md)', () => {
    const r = p.run('remove', 'T001', '--force');
    assert.match(r.stdout, /Removed T001/);
    assert.match(r.stdout, /Updated status.yaml/);
    assert.doesNotMatch(r.stdout, /Deleted/);

    const s = p.run('status');
    assert.doesNotMatch(s.stdout, /T001/);

    assert.ok(p.exists('tasks/T001-task-1.md'));
  });

  it('--all --force removes from status.yaml and deletes .md', () => {
    const r = p.run('remove', 'T002', '--all', '--force');
    assert.match(r.stdout, /Removed T002/);
    assert.match(r.stdout, /Deleted T002-task-2\.md/);

    assert.ok(!p.exists('tasks/T002-task-2.md'));

    const s = p.run('status');
    assert.doesNotMatch(s.stdout, /T002/);
  });
});

describe('taskpad remove — confirmation prompt', () => {
  let p;

  before(() => {
    p = createProject({ tasks: ['T001', 'T002'] });
  });
  after(() => p.destroy());

  it('removes on y response', () => {
    const r = p.runInteractive('remove', 'T001', 'y\n');
    assert.match(r.stdout, /Are you sure/);
    assert.match(r.stdout, /Removed T001/);

    const s = p.run('status');
    assert.doesNotMatch(s.stdout, /T001/);
  });

  it('--all removes .md on y confirmation', () => {
    const r = p.runInteractive('remove', 'T002', '--all', 'y\n');
    assert.match(r.stdout, /Are you sure/);
    assert.match(r.stdout, /Deleted/);
    assert.ok(!p.exists('tasks/T002-task-2.md'));

    const s = p.run('status');
    assert.doesNotMatch(s.stdout, /T002/);
  });

  it('does nothing on N response', () => {
    const p2 = createProject({ tasks: ['T003'] });

    const r = p2.runInteractive('remove', 'T003', 'N\n');
    assert.match(r.stdout, /Are you sure/);

    const s = p2.run('status');
    assert.match(s.stdout, /T003/);

    assert.ok(p2.exists('tasks/T003-task-3.md'));
    p2.destroy();
  });

  it('does nothing on empty response', () => {
    const p3 = createProject({ tasks: ['T004'] });

    const r = p3.runInteractive('remove', 'T004', '\n');
    assert.match(r.stdout, /Are you sure/);

    const s = p3.run('status');
    assert.match(s.stdout, /T004/);
    p3.destroy();
  });
});

describe('taskpad remove — dependent warning', () => {
  let p;

  before(() => {
    p = createProject({
      tasks: [
        'T001',
        { id: 'T002', depends: ['T001'] },
      ],
    });
  });
  after(() => p.destroy());

  it('warns about dependents before removing', () => {
    const r = p.run('remove', 'T001', '--force');
    assert.match(r.stderr, /depend on T001/);
    assert.match(r.stdout, /Removed T001/);
  });
});

describe('taskpad remove — errors', () => {
  let p;

  before(() => {
    p = createProject({ tasks: ['T001'] });
  });
  after(() => p.destroy());

  it('returns error for nonexistent task', () => {
    const r = p.run('remove', 'T999');
    assert.match(r.stderr, /T999 not found/);
    assert.equal(r.status, 1);
  });

  it('returns error for invalid task ID format', () => {
    const r = p.run('remove', 'abc');
    assert.match(r.stderr, /Invalid task ID format/);
    assert.equal(r.status, 1);
  });
});

describe('taskpad remove — no status.yaml', () => {
  let p;

  before(() => {
    p = createProject({ empty: true });
  });
  after(() => p.destroy());

  it('returns error when no tasks directory exists', () => {
    const r = p.run('remove', 'T001', '--force');
    assert.match(r.stderr, /No status.yaml found/);
    assert.equal(r.status, 1);
  });
});

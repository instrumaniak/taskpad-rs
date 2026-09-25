import { describe, it, before, after } from 'node:test';
import assert from 'node:assert/strict';
import { createProject } from '../helpers.mjs';

describe('taskpad do — basic', () => {
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

  it('starts a pending task and changes status to in_progress', () => {
    const r = p.run('do', 'T002', '--force');
    assert.match(r.stdout, /Started T002 — Second Task/);
    assert.match(r.stdout, /Status changed: pending.*in_progress/);
  });
});

describe('taskpad do — already in_progress error', () => {
  let p;

  before(() => {
    p = createProject({
      tasks: [
        { id: 'T001', name: 'First Task' },
      ],
    });
    p.run('do', 'T001', '--force');
  });
  after(() => p.destroy());

  it('returns error when task is already in_progress', () => {
    const r = p.run('do', 'T001');
    assert.match(r.stderr, /already in_progress/);
    assert.equal(r.status, 1);
  });
});

describe('taskpad do — unmet dependencies error', () => {
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

  it('returns error without --force when deps are not met', () => {
    const r = p.run('do', 'T002');
    assert.match(r.stderr, /Unmet dependencies/);
    assert.equal(r.status, 1);
  });
});

describe('taskpad do — --force bypasses dependency check', () => {
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

  it('succeeds with --force even when deps are not met', () => {
    const r = p.run('do', 'T002', '--force');
    assert.match(r.stdout, /Started T002 — Second Task/);
    assert.equal(r.status, 0);
  });
});

describe('taskpad do — invalid task id', () => {
  let p;

  before(() => {
    p = createProject({ empty: true });
  });
  after(() => p.destroy());

  it('returns error for invalid task ID format', () => {
    const r = p.run('do', 'BAD', '--force');
    assert.match(r.stderr, /Invalid task ID format/);
    assert.equal(r.status, 1);
  });
});

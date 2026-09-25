import { describe, it, before, after } from 'node:test';
import assert from 'node:assert/strict';
import { createProject } from '../helpers.mjs';

describe('taskpad import — basic', () => {
  let p;

  before(() => {
    p = createProject({
      importTasks: [
        { id: 'T001', name: 'First Task' },
        { id: 'T002', name: 'Second Task', depends: ['T001'] },
      ],
    });
  });
  after(() => p.destroy());

  it('creates status.yaml from existing T*.md files', () => {
    const r = p.run('import');
    assert.match(r.stdout, /Created status.yaml/);
    assert.match(r.stdout, /Found 2 task files/);
    assert.ok(p.exists('tasks/status.yaml'));
  });

  it('status.yaml contains both tasks with defaults', () => {
    const yaml = p.readFile('tasks/status.yaml');
    assert.match(yaml, /T001/);
    assert.match(yaml, /T002/);
    assert.match(yaml, /First Task/);
    assert.match(yaml, /Second Task/);
    assert.match(yaml, /phase: 0/);
    assert.match(yaml, /critical: false/);
  });
});

describe('taskpad import — --force overwrites', () => {
  let p;

  before(() => {
    p = createProject({
      importTasks: [
        { id: 'T001', name: 'Force Task' },
      ],
    });
  });
  after(() => p.destroy());

  it('fails without --force when status.yaml exists', () => {
    p.run('import');
    const r = p.run('import');
    assert.match(r.stderr, /status\.yaml already exists\. Use --force to overwrite/);
    assert.equal(r.status, 1);
  });

  it('succeeds with --force', () => {
    const r = p.run('import', '--force');
    assert.match(r.stdout, /Overwrote status.yaml/);
    assert.equal(r.status, 0);
  });
});

describe('taskpad import — phase parsing', () => {
  it('parses ## Phase: N correctly', () => {
    const p = createProject({
      importTasks: [
        { id: 'T001', name: 'Phase Test', phase: 3 },
      ],
    });
    p.run('import');
    const yaml = p.readFile('tasks/status.yaml');
    assert.match(yaml, /phase: 3/);
    p.destroy();
  });

  it('defaults to 0 when no ## Phase header', () => {
    const p = createProject({
      importTasks: [
        { id: 'T001', name: 'No Phase' },
      ],
    });
    p.run('import');
    const yaml = p.readFile('tasks/status.yaml');
    assert.match(yaml, /phase: 0/);
    p.destroy();
  });
});

describe('taskpad import — critical parsing', () => {
  it('parses ## Critical: true', () => {
    const p = createProject({
      importTasks: [
        { id: 'T001', name: 'Critical True', critical: true },
      ],
    });
    p.run('import');
    const yaml = p.readFile('tasks/status.yaml');
    assert.match(yaml, /critical: true/);
    p.destroy();
  });

  it('defaults to false when no ## Critical header', () => {
    const p = createProject({
      importTasks: [
        { id: 'T001', name: 'Not Critical' },
      ],
    });
    p.run('import');
    const yaml = p.readFile('tasks/status.yaml');
    assert.match(yaml, /critical: false/);
    p.destroy();
  });
});

describe('taskpad import — status parsing', () => {
  it('parses ## Status: done', () => {
    const p = createProject({
      importTasks: [
        { id: 'T001', name: 'Done Task', status: 'done' },
      ],
    });
    p.run('import');
    const yaml = p.readFile('tasks/status.yaml');
    assert.match(yaml, /status: done/);
    p.destroy();
  });

  it('defaults to pending when no ## Status header', () => {
    const p = createProject({
      importTasks: [
        { id: 'T001', name: 'Pending Task' },
      ],
    });
    p.run('import');
    const yaml = p.readFile('tasks/status.yaml');
    assert.match(yaml, /status: pending/);
    p.destroy();
  });
});

describe('taskpad import — depends parsing', () => {
  it('parses ## Depends On references', () => {
    const p = createProject({
      importTasks: [
        { id: 'T001', name: 'First Task' },
        { id: 'T002', name: 'Dep Task', depends: ['T001'] },
      ],
    });
    p.run('import');
    const yaml = p.readFile('tasks/status.yaml');
    assert.match(yaml, /T002[\s\S]*depends:\n.*- T001/);
    p.destroy();
  });
});

describe('taskpad import — missing sections default', () => {
  it('handles task with no optional headers', () => {
    const p = createProject({
      importTasks: [
        { id: 'T001', name: 'Minimal Task' },
      ],
    });
    p.run('import');
    const yaml = p.readFile('tasks/status.yaml');
    assert.match(yaml, /phase: 0/);
    assert.match(yaml, /critical: false/);
    assert.match(yaml, /status: pending/);
    p.destroy();
  });
});

describe('taskpad import — errors', () => {
  it('returns error for circular dependency', () => {
    const p = createProject({
      importTasks: [
        { id: 'T001', name: 'Task A', depends: ['T002'] },
        { id: 'T002', name: 'Task B', depends: ['T001'] },
      ],
    });
    const r = p.run('import');
    assert.match(r.stderr, /warning: \d+ issue\(s\) found/);
    assert.match(r.stderr, /circular/i);
    assert.equal(r.status, 0); // still writes with warning
    assert.ok(p.exists('tasks/status.yaml'));
    p.destroy();
  });

  it('reports error when dependency not found', () => {
    const p = createProject({
      importTasks: [
        { id: 'T001', name: 'Missing Dep', depends: ['T999'] },
      ],
    });
    const r = p.run('import');
    assert.match(r.stderr, /warning: \d+ issue\(s\) found/);
    assert.match(r.stderr, /T999/);
    assert.equal(r.status, 0); // still writes with warning
    assert.ok(p.exists('tasks/status.yaml'));
    p.destroy();
  });

  it('reports no files when no T*.md files exist', () => {
    const p = createProject({ empty: true });
    const r = p.run('import');
    assert.match(r.stdout, /No T\*\.md files found/);
    assert.equal(r.status, 0);
    p.destroy();
  });
});

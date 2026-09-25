import { describe, it, before, after } from 'node:test';
import assert from 'node:assert/strict';
import { createProject } from '../helpers.mjs';

describe('taskpad edit — task-level status change', () => {
  let p;

  before(() => {
    p = createProject({
      tasks: [
        { id: 'T001', name: 'First Task' },
        { id: 'T002', name: 'Second Task' },
        { id: 'T003', name: 'Third Task' },
      ],
    });
  });
  after(() => p.destroy());

  it('updates the status and prints one confirmation line', () => {
    const r = p.run('edit', 'T001', '--status', 'in_progress');
    assert.equal(r.stdout, 'Updated T001 status: in_progress');
    assert.equal(r.stderr, '');
    assert.equal(r.status, 0);
  });

  it('persists the new status to status.yaml', () => {
    const yaml = p.readFile('tasks/status.yaml');
    assert.match(yaml, /T001:[\s\S]*?status: in_progress/);
  });
});

describe('taskpad edit — combined task-level flags', () => {
  let p;

  before(() => {
    p = createProject({
      tasks: [
        { id: 'T001', name: 'First Task' },
        { id: 'T002', name: 'Second Task' },
        { id: 'T003', name: 'Third Task' },
      ],
    });
  });
  after(() => p.destroy());

  it('applies status, depends, phase, and critical in one command', () => {
    const r = p.run(
      'edit', 'T001',
      '--status', 'done',
      '--phase', '2',
      '--critical',
      '--depends', 'T002',
    );
    assert.equal(
      r.stdout,
      'Updated T001 status: done depends: T002 phase: 2 critical: true',
    );
    assert.equal(r.status, 0);
  });

  it('persists every changed field to status.yaml', () => {
    const yaml = p.readFile('tasks/status.yaml');
    assert.match(yaml, /T001:[\s\S]*?status: done/);
    assert.match(yaml, /T001:[\s\S]*?depends:[\s\S]{0,20}T002/);
    assert.match(yaml, /T001:[\s\S]*?phase: 2/);
    assert.match(yaml, /T001:[\s\S]*?critical: true/);
  });
});

describe('taskpad edit — repeated --depends', () => {
  let p;

  before(() => {
    p = createProject({
      tasks: [
        { id: 'T001', name: 'First Task' },
        { id: 'T002', name: 'Second Task' },
        { id: 'T003', name: 'Third Task' },
      ],
    });
  });
  after(() => p.destroy());

  it('replaces depends with every supplied id, joined with ", "', () => {
    const r = p.run(
      'edit', 'T001',
      '--depends', 'T002',
      '--depends', 'T003',
    );
    assert.equal(r.stdout, 'Updated T001 depends: T002, T003');
    assert.equal(r.status, 0);
  });
});

describe('taskpad edit — --critical and --no-critical', () => {
  let p;

  before(() => {
    p = createProject({
      tasks: [{ id: 'T001', name: 'First Task' }],
    });
  });
  after(() => p.destroy());

  it('--critical marks the task critical', () => {
    const r = p.run('edit', 'T001', '--critical');
    assert.equal(r.stdout, 'Updated T001 critical: true');
    assert.equal(r.status, 0);
    assert.match(p.readFile('tasks/status.yaml'), /critical: true/);
  });

  it('--no-critical unsets it', () => {
    const r = p.run('edit', 'T001', '--no-critical');
    assert.equal(r.stdout, 'Updated T001 critical: false');
    assert.equal(r.status, 0);
    assert.match(p.readFile('tasks/status.yaml'), /critical: false/);
  });

  it('both flags together behave like --critical (cli.cpp: critVal = editCritical)', () => {
    const r = p.run('edit', 'T001', '--critical', '--no-critical');
    assert.equal(r.stdout, 'Updated T001 critical: true');
    assert.equal(r.status, 0);
    assert.match(p.readFile('tasks/status.yaml'), /critical: true/);
  });
});

describe('taskpad edit — invalid status', () => {
  let p;

  before(() => {
    p = createProject({
      tasks: [{ id: 'T001', name: 'First Task' }],
    });
  });
  after(() => p.destroy());

  it('rejects an unknown status value', () => {
    const r = p.run('edit', 'T001', '--status', 'bogus');
    assert.match(r.stderr, /Invalid status\. Must be: pending, in_progress, or done/);
    assert.equal(r.status, 1);
  });

  it('leaves status.yaml untouched on failure', () => {
    const yaml = p.readFile('tasks/status.yaml');
    assert.match(yaml, /T001:[\s\S]*?status: pending/);
  });
});

describe('taskpad edit — invalid task id', () => {
  let p;

  before(() => {
    // edit reads status.yaml before validating the id (commands.cpp:889
    // runs readStatusFile ahead of the isValidTaskId check), so the
    // fixture needs a status.yaml for the id error to surface.
    p = createProject({
      tasks: [{ id: 'T001', name: 'First Task' }],
    });
  });
  after(() => p.destroy());

  it('returns error for invalid task ID format', () => {
    const r = p.run('edit', 'BAD', '--status', 'done');
    assert.match(r.stderr, /Invalid task ID format\. Expected TXXX \(see Task ID Format\)/);
    assert.equal(r.status, 1);
  });
});

describe('taskpad edit — task not found', () => {
  let p;

  before(() => {
    p = createProject({
      tasks: [{ id: 'T001', name: 'First Task' }],
    });
  });
  after(() => p.destroy());

  it('returns error when task does not exist', () => {
    const r = p.run('edit', 'T999', '--status', 'done');
    assert.match(r.stderr, /Task T999 not found/);
    assert.equal(r.status, 1);
  });
});

describe('taskpad edit — no changes specified', () => {
  let p;

  before(() => {
    p = createProject({
      tasks: [{ id: 'T001', name: 'First Task' }],
    });
  });
  after(() => p.destroy());

  it('errors when a task id is given without any task-level flag', () => {
    const r = p.run('edit', 'T001');
    assert.match(
      r.stderr,
      /No changes specified\. Use --status, --phase, --critical, or --depends/,
    );
    assert.equal(r.status, 1);
  });

  it('project-level flags are ignored in the task-level branch', () => {
    const r = p.run('edit', 'T001', '--phases', '0:Scaffolding');
    assert.match(
      r.stderr,
      /No changes specified\. Use --status, --phase, --critical, or --depends/,
    );
    assert.equal(r.status, 1);
  });
});

describe('taskpad edit — phase validation', () => {
  let p;

  before(() => {
    p = createProject({
      tasks: [{ id: 'T001', name: 'First Task' }],
    });
  });
  after(() => p.destroy());

  it('rejects a negative phase', () => {
    const r = p.run('edit', 'T001', '--phase', '-1');
    assert.match(r.stderr, /Phase must be non-negative/);
    assert.equal(r.status, 1);
  });

  // The C++ binary aborts (uncaught std::invalid_argument from std::stoi)
  // on a non-numeric phase; the port turns that into a clean error instead
  // of panicking — see spec.main.md §6 "No panics" and the T012 task Notes.
  it('rejects a non-numeric phase', () => {
    const r = p.run('edit', 'T001', '--phase', 'abc');
    assert.match(r.stderr, /Invalid phase\. Must be a non-negative integer/);
    assert.equal(r.status, 1);
  });
});

describe('taskpad edit — dependency errors', () => {
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

  it('rejects a dependency that does not exist', () => {
    const r = p.run('edit', 'T002', '--depends', 'T999');
    assert.match(r.stderr, /Dependency T999 not found/);
    assert.equal(r.status, 1);
  });

  it('rejects a self-dependency', () => {
    const r = p.run('edit', 'T001', '--depends', 'T001');
    assert.match(r.stderr, /Circular dependency detected: T001 depends on itself/);
    assert.equal(r.status, 1);
  });

  it('rejects a transitive cycle with the literal → ... → separator', () => {
    // T002 already depends on T001, so T001 -> T002 would close the loop.
    const r = p.run('edit', 'T001', '--depends', 'T002');
    assert.match(r.stderr, /Circular dependency detected: T001 → \.\.\. → T002/);
    assert.equal(r.status, 1);
  });

  it('does not write status.yaml when validation fails', () => {
    // The helpers.mjs fixture style uses `depends: []`, which a failed
    // edit must leave exactly as it was.
    const yaml = p.readFile('tasks/status.yaml');
    assert.match(yaml, /T001:[\s\S]*?depends: \[\]/);
  });
});

describe('taskpad edit — project-level phases', () => {
  let p;

  before(() => {
    p = createProject({
      tasks: [
        { id: 'T001', name: 'First Task' },
        { id: 'T002', name: 'Second Task' },
      ],
    });
  });
  after(() => p.destroy());

  it('replaces the phase mapping and confirms', () => {
    const r = p.run('edit', '--phases', '0:Scaffolding,1:Foundation');
    assert.equal(r.stdout, 'Updated phases mapping');
    assert.equal(r.status, 0);
  });

  it('persists both phase entries to status.yaml', () => {
    const yaml = p.readFile('tasks/status.yaml');
    assert.match(yaml, /phases:/);
    assert.match(yaml, /Scaffolding/);
    assert.match(yaml, /Foundation/);
  });

  it('replaces rather than merges on a second run', () => {
    const r = p.run('edit', '--phases', '7:Only');
    assert.equal(r.stdout, 'Updated phases mapping');
    assert.equal(r.status, 0);
    const yaml = p.readFile('tasks/status.yaml');
    assert.match(yaml, /phases:[\s\S]{0,40}7: Only/);
    assert.doesNotMatch(yaml, /Scaffolding/);
  });
});

describe('taskpad edit — project-level critical path', () => {
  let p;

  before(() => {
    p = createProject({
      tasks: [
        { id: 'T001', name: 'First Task' },
        { id: 'T002', name: 'Second Task' },
      ],
    });
  });
  after(() => p.destroy());

  it('sets the critical path and confirms', () => {
    const r = p.run('edit', '--critical-path', 'T001,T002');
    assert.equal(r.stdout, 'Updated critical path');
    assert.equal(r.status, 0);
    const yaml = p.readFile('tasks/status.yaml');
    assert.match(yaml, /critical_path:[\s\S]{0,40}T001/);
    assert.match(yaml, /critical_path:[\s\S]{0,60}T002/);
  });

  it('rejects an id that is not a task', () => {
    const r = p.run('edit', '--critical-path', 'T999');
    assert.match(r.stderr, /Task T999 in critical path not found/);
    assert.equal(r.status, 1);
  });
});

describe('taskpad edit — project-level combined flags', () => {
  let p;

  before(() => {
    p = createProject({
      tasks: [
        { id: 'T001', name: 'First Task' },
        { id: 'T002', name: 'Second Task' },
      ],
    });
  });
  after(() => p.destroy());

  it('prints one confirmation line per supplied flag', () => {
    const r = p.run(
      'edit',
      '--phases', '0:Scaffolding',
      '--critical-path', 'T001,T002',
    );
    assert.equal(r.stdout, 'Updated phases mapping\nUpdated critical path');
    assert.equal(r.status, 0);
  });

  it('persists both to status.yaml', () => {
    const yaml = p.readFile('tasks/status.yaml');
    assert.match(yaml, /Scaffolding/);
    assert.match(yaml, /critical_path:/);
  });
});

describe('taskpad edit — no project-level changes', () => {
  let p;

  before(() => {
    p = createProject({
      tasks: [{ id: 'T001', name: 'First Task' }],
    });
  });
  after(() => p.destroy());

  it('errors when no flags are given at all', () => {
    const r = p.run('edit');
    assert.match(
      r.stderr,
      /No project-level changes specified\. Use --phases or --critical-path/,
    );
    assert.equal(r.status, 1);
  });

  it('task-level flags are ignored in the project-level branch', () => {
    const r = p.run('edit', '--status', 'done');
    assert.match(
      r.stderr,
      /No project-level changes specified\. Use --phases or --critical-path/,
    );
    assert.equal(r.status, 1);
  });
});

describe('taskpad edit — missing status.yaml', () => {
  let p;

  before(() => {
    p = createProject({ empty: true });
  });
  after(() => p.destroy());

  it('task-level branch fails before validating the id', () => {
    const r = p.run('edit', 'T001', '--status', 'done');
    assert.match(
      r.stderr,
      /No status\.yaml found\. Run 'taskpad import' or 'taskpad new' first/,
    );
    assert.equal(r.status, 1);
  });

  it('project-level branch fails the same way', () => {
    const r = p.run('edit', '--phases', '0:Scaffolding');
    assert.match(
      r.stderr,
      /No status\.yaml found\. Run 'taskpad import' or 'taskpad new' first/,
    );
    assert.equal(r.status, 1);
  });
});

// E2E deferred to T015 (per AGENTS.md)

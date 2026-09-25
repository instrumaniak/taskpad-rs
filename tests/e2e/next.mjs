import { describe, it, before, after } from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import { createProject } from '../helpers.mjs';

describe('taskpad next', () => {
  let p;

  before(() => {
    p = createProject({
      importTasks: [
        { id: 'T001', name: 'First Task' },
        { id: 'T002', name: 'Second Task', depends: ['T001'] },
      ],
    });
    // T001 has no deps, T002 depends on T001
    // Import, then mark T001 as done to unblock T002
    p.run('import');
    p.run('do', 'T001', '--force');
    p.run('done', 'T001');
  });
  after(() => p.destroy());

  it('shows next task when one is available', () => {
    const r = p.run('next');
    assert.match(r.stdout, /Next:/);
    assert.match(r.stdout, /T002/);
  });

  it('reads files and specs from T*.md via extractSectionListItems', () => {
    const r = p.run('next');
    assert.match(r.stdout, /Files:/);
    assert.match(r.stdout, /src\/second-task\.cpp/);
    assert.match(r.stdout, /Specs:/);
    assert.match(r.stdout, /specs\/spec1\.md/);
  });
});

describe('taskpad next — all blocked or complete', () => {
  let p;

  before(() => {
    p = createProject({
      importTasks: [
        { id: 'T001', name: 'Blocked Task', depends: ['T999'] },
      ],
    });
    p.run('import');
  });
  after(() => p.destroy());

  it('shows info when all tasks blocked or complete', () => {
    const r = p.run('next');
    assert.match(r.stdout, /All tasks blocked or complete/);
  });
});

describe('taskpad next — priority order', () => {
  let p;

  before(() => {
    p = createProject({
      importTasks: [
        { id: 'T001', name: 'Phase 1 Non-Critical', phase: 1, critical: false },
        { id: 'T002', name: 'Phase 0 Critical', phase: 0, critical: true },
        { id: 'T003', name: 'Phase 0 Non-Critical', phase: 0, critical: false },
      ],
    });
    p.run('import');
  });
  after(() => p.destroy());

  it('prioritizes critical path over phase number', () => {
    const r = p.run('next');
    assert.match(r.stdout, /T002/);
    assert.match(r.stdout, /Phase 0 Critical/);
  });
});

describe('taskpad next — missing T*.md file', () => {
  let p;

  before(() => {
    // statusYaml mode writes only status.yaml — no T*.md files exist.
    p = createProject({
      statusYaml: `tasks:
  T001:
    name: "Fileless Task"
    status: pending
    depends: []
    phase: 0
    critical: false
`,
    });
  });
  after(() => p.destroy());

  it('silently omits file-derived detail and creates no template', () => {
    const r = p.run('next');
    assert.match(r.stdout, /Next:/);
    assert.match(r.stdout, /T001/);
    assert.match(r.stdout, /Fileless Task/);
    assert.doesNotMatch(r.stdout, /Goal:/);
    assert.doesNotMatch(r.stdout, /First step:/);
    assert.equal(r.stderr, '');
    assert.equal(r.status, 0);
    // No template file is created for the missing T*.md (spec.main.md §6).
    assert.deepEqual(fs.readdirSync(p.resolve('tasks')), ['status.yaml']);
  });
});

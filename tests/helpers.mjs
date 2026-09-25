import { execSync, spawnSync } from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';
import os from 'node:os';

const ROOT = process.cwd();
const BINARY = path.join(ROOT, 'target', 'release', 'taskpad');
const BUILD_FLAG = path.join(os.tmpdir(), '.taskpad-rs-e2e-built');

function ensureBuilt() {
  if (fs.existsSync(BUILD_FLAG)) return;
  execSync('cargo build --release', { stdio: 'inherit', cwd: ROOT });
  fs.writeFileSync(BUILD_FLAG, '');
}

function taskDefaults(id) {
  const n = parseInt(id.slice(1), 10);
  return {
    id,
    name: `Task ${n}`,
    status: 'pending',
    depends: [],
    phase: 0,
    critical: false,
  };
}

function toKebab(s) {
  return s.toLowerCase().replace(/\s+/g, '-');
}

function taskFilename(id, name) {
  return `${id}-${toKebab(name)}.md`;
}

function mdImportContent(id, name, { phase, critical, depends, status } = {}) {
  const depLine = depends && depends.length > 0 ? depends.join(', ') : '(None)';
  const lines = [
    `# ${id}: ${name}`,
    '',
    '## Goal',
    '',
    'Test task.',
    '',
    '## Depends On',
    '',
    depLine,
    '',
    '## Spec References',
    '',
    '- `specs/spec1.md`',
    '',
    '## Files to Create/Modify',
    '',
    `- \`src/${toKebab(name)}.cpp\``,
    '',
    '## Implementation Steps',
    '',
    '1. Implement the task',
    '',
    '## Acceptance Criteria',
    '',
    '- [ ] Task complete',
    '',
  ];
  if (phase !== undefined) {
    lines.splice(3, 0, '', `## Phase: ${phase}`);
  }
  if (critical !== undefined) {
    lines.splice(3, 0, '', `## Critical: ${critical}`);
  }
  if (status !== undefined) {
    lines.splice(3, 0, '', `## Status: ${status}`);
  }
  return lines.join('\n');
}

function generateYaml(tasks) {
  let yaml = '# taskpad status file\n\ntasks:\n';
  for (const t of tasks) {
    yaml += `  ${t.id}:\n`;
    yaml += `    name: "${t.name}"\n`;
    yaml += `    status: ${t.status}\n`;
    yaml += `    depends: [${t.depends.join(', ')}]\n`;
    yaml += `    phase: ${t.phase}\n`;
    yaml += `    critical: ${t.critical}\n`;
  }
  return yaml;
}

function mdContent(id, name, depends) {
  const depLine = depends.length > 0 ? depends.join(', ') : '(None)';
  return [
    `# ${id}: ${name}`,
    '',
    '## Goal',
    '',
    'Test task.',
    '',
    '## Depends On',
    '',
    depLine,
    '',
    '## Implementation Steps',
    '',
    '1. Implement the task',
    '',
    '## Acceptance Criteria',
    '',
    '- [ ] Task complete',
    '',
  ].join('\n');
}

export function createProject({ tasks = [], empty = false, statusYaml, importTasks: importTasksOpt } = {}) {
  ensureBuilt();

  const tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), 'taskpad-test-'));
  const projectDir = path.join(tmpDir, 'project');
  const taskDir = 'tasks';

  fs.mkdirSync(path.join(projectDir, taskDir), { recursive: true });
  fs.writeFileSync(
    path.join(projectDir, '.taskpad'),
    `task-dir: ${taskDir}\n`
  );

  if (!empty) {
    const td = path.join(projectDir, taskDir);

    if (statusYaml) {
      fs.writeFileSync(path.join(td, 'status.yaml'), statusYaml);
    } else if (importTasksOpt) {
      for (const t of importTasksOpt) {
        const defaults = taskDefaults(t.id);
        const name = t.name || defaults.name;
        const fname = taskFilename(t.id, name);
        fs.writeFileSync(
          path.join(td, fname),
          mdImportContent(t.id, name, {
            phase: t.phase,
            critical: t.critical,
            depends: t.depends,
            status: t.status,
          })
        );
      }
    } else if (tasks.length > 0) {
      const resolved = tasks.map(t =>
        typeof t === 'string' ? taskDefaults(t) : { ...taskDefaults(t.id), ...t }
      );
      fs.writeFileSync(path.join(td, 'status.yaml'), generateYaml(resolved));

      for (const t of resolved) {
        const fname = taskFilename(t.id, t.name);
        fs.writeFileSync(
          path.join(td, fname),
          mdContent(t.id, t.name, t.depends)
        );
      }
    }
  }

  function run(...args) {
    const r = spawnSync(BINARY, args, {
      cwd: projectDir,
      encoding: 'utf-8',
    });
    return {
      stdout: (r.stdout || '').trim(),
      stderr: (r.stderr || '').trim(),
      status: r.status,
    };
  }

  function runInteractive(...all) {
    const input = all.pop();
    const r = spawnSync(BINARY, all, {
      cwd: projectDir,
      input,
      encoding: 'utf-8',
    });
    return {
      stdout: (r.stdout || '').trim(),
      stderr: (r.stderr || '').trim(),
      status: r.status,
    };
  }

  function exists(relativePath) {
    return fs.existsSync(path.join(projectDir, relativePath));
  }

  function readFile(relativePath) {
    return fs.readFileSync(path.join(projectDir, relativePath), 'utf-8');
  }

  function resolve(relativePath) {
    return path.join(projectDir, relativePath);
  }

  function destroy() {
    fs.rmSync(tmpDir, { recursive: true, force: true });
  }

  return { run, runInteractive, exists, readFile, resolve, destroy };
}

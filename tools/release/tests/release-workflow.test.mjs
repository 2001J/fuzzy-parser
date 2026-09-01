import assert from 'node:assert/strict';
import { copyFile, mkdtemp, mkdir, readFile, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { basename, dirname, resolve } from 'node:path';
import { spawnSync } from 'node:child_process';
import test from 'node:test';
import { fileURLToPath } from 'node:url';

const repositoryRoot = resolve(dirname(fileURLToPath(import.meta.url)), '../../..');
const workflowPath = resolve(repositoryRoot, '.github/workflows/release.yml');
const checksumWriter = resolve(repositoryRoot, 'tools/release/write-checksum.sh');

function jobBlock(workflow, name, nextName) {
  const start = workflow.indexOf(`  ${name}:\n`);
  assert.notEqual(start, -1, `missing ${name} job`);
  const end = nextName ? workflow.indexOf(`  ${nextName}:\n`, start + 1) : workflow.length;
  assert.notEqual(end, -1, `missing ${nextName} job`);
  return workflow.slice(start, end);
}

test('npm provenance receives only job-level OIDC permission', async () => {
  const workflow = await readFile(workflowPath, 'utf8');
  const npmJob = jobBlock(workflow, 'npm', 'container');

  assert.match(npmJob, /\n    permissions:\n      id-token: write\n    steps:/);
  assert.doesNotMatch(npmJob, /contents: write|packages: write|actions: write/);
  assert.match(npmJob, /npm publish .* --provenance/);
  assert.match(npmJob, /if: \$\{\{ inputs\.publish_npm == true \}\}/);
  assert.match(npmJob, /environment: release/);
});

test('CLI and npm artifacts use the portable checksum writer', async () => {
  const workflow = await readFile(workflowPath, 'utf8');
  const cliJob = jobBlock(workflow, 'cli', 'node-package');
  const nodeJob = jobBlock(workflow, 'node-package', 'github-release');

  assert.match(cliJob, /tools\/release\/write-checksum\.sh "\$archive"/);
  assert.match(nodeJob, /tools\/release\/write-checksum\.sh "\$RUNNER_TEMP\/\$archive_name"/);
  assert.doesNotMatch(workflow, /shasum -a 256 "\$RUNNER_TEMP\//);
});

test('checksum files contain a basename and verify after download elsewhere', async (t) => {
  const root = await mkdtemp(resolve(tmpdir(), 'fuzzy-parser-checksum-'));
  t.after(() => rm(root, { recursive: true, force: true }));

  const buildDirectory = resolve(root, 'runner', 'temporary', 'artifacts');
  const downloadDirectory = resolve(root, 'downloaded');
  await mkdir(buildDirectory, { recursive: true });
  await mkdir(downloadDirectory);

  for (const archiveName of [
    'fuzzy-parser-0.1.0-linux-x86_64.tar.gz',
    'fuzzy-parser-node-0.1.0.tgz',
  ]) {
    const archive = resolve(buildDirectory, archiveName);
    await writeFile(archive, `deterministic synthetic artifact: ${archiveName}\n`);

    const writeResult = spawnSync(checksumWriter, [archive], { encoding: 'utf8' });
    assert.equal(writeResult.status, 0, writeResult.stderr);

    const checksum = `${archive}.sha256`;
    const checksumText = await readFile(checksum, 'utf8');
    assert.match(checksumText, new RegExp(`^[0-9a-f]{64}  ${archiveName.replaceAll('.', '\\.')}\\n$`));
    assert.equal(checksumText.includes(buildDirectory), false);

    await copyFile(archive, resolve(downloadDirectory, basename(archive)));
    await copyFile(checksum, resolve(downloadDirectory, basename(checksum)));
    const verifyResult = spawnSync('shasum', ['-a', '256', '--check', basename(checksum)], {
      cwd: downloadDirectory,
      encoding: 'utf8',
    });
    assert.equal(verifyResult.status, 0, verifyResult.stderr);
    assert.match(verifyResult.stdout, /: OK\n$/);
  }
});

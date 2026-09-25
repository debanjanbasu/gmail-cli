import { readFile, writeFile } from 'node:fs/promises';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const scriptDirectory = dirname(fileURLToPath(import.meta.url));
const repositoryRoot = resolve(scriptDirectory, '..', '..');
const cargoManifestPath = resolve(repositoryRoot, 'Cargo.toml');
const statsPath = resolve(repositoryRoot, 'site', 'src', 'data', 'stats.json');
const userAgent = 'grr-cli-site-stats/0.4.0 (+https://github.com/debanjanbasu/grr-cli)';

const readPackageVersion = async () => {
  const manifest = await readFile(cargoManifestPath, 'utf8');
  const packageSection = manifest.match(/\[package\]([\s\S]*?)(?:\n\[|$)/)?.[1] ?? '';
  const version = packageSection.match(/^\s*version\s*=\s*"([^"]+)"/m)?.[1];
  if (!version) {
    throw new Error(`package version not found in ${cargoManifestPath}`);
  }
  return version;
};

const fetchJson = async (url, headers = {}) => {
  const response = await fetch(url, {
    headers: {
      Accept: 'application/json',
      'User-Agent': userAgent,
      ...headers,
    },
    signal: AbortSignal.timeout(15000),
  });
  if (!response.ok) {
    throw new Error(`${url} returned HTTP ${response.status}`);
  }
  return response.json();
};

const asCount = (value) => {
  const count = Number(value);
  return Number.isFinite(count) && count >= 0 ? Math.trunc(count) : 0;
};

try {
  const [version, cratePayload, releasePayload] = await Promise.all([
    readPackageVersion(),
    fetchJson('https://crates.io/api/v1/crates/grr-cli'),
    fetchJson('https://api.github.com/repos/debanjanbasu/grr-cli/releases', {
      Accept: 'application/vnd.github+json',
      'X-GitHub-Api-Version': '2022-11-28',
    }),
  ]);

  const crate = cratePayload?.crate;
  if (!crate || !Array.isArray(releasePayload)) {
    throw new Error('unexpected API response shape');
  }

  const releases = releasePayload
    .filter((release) => release && !release.draft)
    .sort((left, right) => {
      const leftDate = Date.parse(left.published_at ?? left.created_at ?? '') || 0;
      const rightDate = Date.parse(right.published_at ?? right.created_at ?? '') || 0;
      return rightDate - leftDate;
    });
  const latestRelease = releases[0];
  const githubDownloads = releases.reduce(
    (total, release) => total + (Array.isArray(release.assets) ? release.assets.reduce((sum, asset) => sum + asCount(asset?.download_count), 0) : 0),
    0,
  );

  const stats = {
    version,
    crateDownloads: asCount(crate.downloads),
    crateDownloads30d: asCount(crate.recent_downloads),
    githubDownloads,
    latestRelease: latestRelease?.tag_name ?? '',
    latestReleaseDate: latestRelease?.published_at ?? latestRelease?.created_at ?? '',
    updatedAt: new Date().toISOString(),
  };

  await writeFile(statsPath, `${JSON.stringify(stats, null, 2)}\n`, 'utf8');
  console.log(`Updated ${statsPath}: ${stats.crateDownloads} crates.io downloads, ${stats.githubDownloads} GitHub asset downloads`);
} catch (error) {
  const message = error instanceof Error ? error.message : String(error);
  console.warn(`Stats refresh skipped: ${message}`);
  console.warn(`Keeping the existing ${statsPath}`);
}

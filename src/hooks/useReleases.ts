import { useState, useCallback } from 'react';
import { message } from '../antdStatic';
import { api, GitHubRelease } from '../api';
import { getErrorMessage } from '../utils';

const CACHE_KEY = 'astrbot_releases_cache';
const CACHE_DURATION = 8 * 60 * 60 * 1000; // 8 hours

interface ReleasesCache {
  releases: GitHubRelease[];
  timestamp: number;
}

function loadCachedReleases(): GitHubRelease[] | null {
  try {
    const cached = localStorage.getItem(CACHE_KEY);
    if (!cached) return null;

    const data: ReleasesCache = JSON.parse(cached);
    const now = Date.now();

    if (now - data.timestamp < CACHE_DURATION) {
      return data.releases;
    }
    return null;
  } catch {
    return null;
  }
}

function saveCachedReleases(releases: GitHubRelease[]): void {
  try {
    const cache: ReleasesCache = {
      releases,
      timestamp: Date.now(),
    };
    localStorage.setItem(CACHE_KEY, JSON.stringify(cache));
  } catch {
    // Ignore storage errors
  }
}

/**
 * Hook for fetching and caching GitHub releases.
 */
export function useReleases() {
  const [releases, setReleases] = useState<GitHubRelease[]>([]);
  const [loading, setLoading] = useState(false);

  const fetchReleases = useCallback(async (forceRefresh = false) => {
    // Try cache first
    if (!forceRefresh) {
      const cached = loadCachedReleases();
      if (cached) {
        setReleases(cached);
        return cached;
      }
    }

    setLoading(true);
    try {
      const r = await api.fetchReleases();
      setReleases(r);
      saveCachedReleases(r);
      return r;
    } catch (e: unknown) {
      message.error(getErrorMessage(e));
      return [];
    } finally {
      setLoading(false);
    }
  }, []);

  return { releases, loading, fetchReleases, setReleases };
}

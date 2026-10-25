interface UnghRelease {
  id: number;
  tag: string;
  author: string;
  name: string;
  draft: boolean;
  prerelease: boolean;
  createdAt: string;
  publishedAt: string;
  markdown: string;
  html: string;
}

export interface Release {
  tag: string;
  title: string;
  date: string;
  prerelease: boolean;
  markdown: string;
}

export function useReleases(channel: MaybeRef<"stable" | "preview">) {
  const appConfig = useAppConfig();
  const isDev = import.meta.dev;

  const url = computed(() =>
    isDev ? "/__dev/releases" : `https://ungh.cc/repos/${appConfig.repository}/releases`,
  );

  return useFetch(url, {
    key: `releases-${toValue(channel)}`,
    transform(data: { releases: UnghRelease[] }) {
      return data.releases
        .filter((r) => !r.draft)
        .filter((r) => (toValue(channel) === "preview" ? r.prerelease : !r.prerelease))
        .map((r) => ({
          tag: r.tag,
          title: r.name || r.tag,
          date: r.publishedAt,
          prerelease: r.prerelease,
          markdown: r.markdown,
        }))
        .sort((a, b) => new Date(b.date).getTime() - new Date(a.date).getTime());
    },
  });
}

<script setup lang="ts">
  import type { ButtonProps, ThemeUI } from "@nuxt/ui";
  import { createReusableTemplate } from "@vueuse/core";

  const { data: roadmap } = await useAsyncData("roadmap", () => queryCollection("roadmap").first());
  type RoadmapStatus = "done" | "in-progress" | "next" | "later";
  type RoadmapItem = {
    title: string;
    description: string;
    status: RoadmapStatus;
  };

  // "done" is not a column of its own — shipped work is folded into the
  // "in-progress" column below.
  type RoadmapColumnKey = Exclude<RoadmapStatus, "done">;

  const grouped = computed(() => {
    const groups: Record<RoadmapColumnKey, RoadmapItem[]> = {
      "in-progress": [],
      next: [],
      later: [],
    };
    const done: RoadmapItem[] = [];

    for (const item of roadmap.value?.items ?? []) {
      if (item.status === "done") done.push(item);
      else groups[item.status]?.push(item);
    }

    // Appended rather than interleaved, so in-flight work stays at the top.
    groups["in-progress"].push(...done);

    return groups;
  });

  const badges = {
    done: { label: "Done", color: "success" as const },
    "in-progress": { label: "In Progress", color: "primary" as const },
    next: { label: "Next", color: "secondary" as const },
    later: { label: "Future", color: "info" as const },
  };
  type RoadmapColumn = {
    key: RoadmapColumnKey;
    label: string;
  };

  const columns: Record<RoadmapColumnKey, RoadmapColumn> = {
    "in-progress": {
      key: "in-progress",
      label: "Currently working on",
    },
    next: {
      key: "next",
      label: "Coming up next",
    },
    later: {
      key: "later",
      label: "Longer term",
    },
  };

  const channels: ButtonProps[] = [
    {
      label: "Contribute",
      to: "https://github.com/waypoints-hq/waypoints",
      target: "_blank",
      trailingIcon: "i-lucide-arrow-right",
    },
    {
      label: "Issues",
      to: "https://github.com/waypoints-hq/waypoints/issues",
      target: "_blank",
      icon: "i-simple-icons-github",
      color: "neutral",
      variant: "subtle",
    },
  ];

  const [DefineRoadmapGroup, ReuseRoadmapGroup] = createReusableTemplate<{
    group: RoadmapColumn;
    orientation: "horizontal" | "vertical";
  }>();
  const [DefineRoadmapItem, ReuseRoadmapItem] = createReusableTemplate<{
    item: RoadmapItem;
    badge: (typeof badges)[RoadmapStatus];
  }>();

  const theming: ThemeUI = {
    page: {
      root: "md:border-x border-default",
    },
    pageHero: {
      root: "md:border-b border-default",
      container: "relative py-10 sm:py-16 lg:py-24 md:border-x border-default",
    },
    pageBody: {
      base: "divide-y divide-default",
    },
    pageSection: {
      root: "divide-inherit py-0",
      container: "py-4 sm:py-6 lg:py-8 gap-4 sm:gap-8",
    },
    pageList: {
      base: "space-y-3",
    },
    pageCard: {
      wrapper: "flex-row-reverse items-center",
      leading: "mb-0",
      container: "gap-y-1 text-sm text-balance",
    },
  };
</script>

<template>
  <DefineRoadmapItem v-slot="{ item, badge }">
    <UPageCard :key="item.title" :title="item.title" spotlight :spotlight-color="badge.color">
      <template v-if="badge" #leading>
        <LazyUBadge size="sm" v-bind="badge" />
      </template>
      {{ item.description }}
    </UPageCard>
  </DefineRoadmapItem>

  <DefineRoadmapGroup v-slot="{ group, orientation }">
    <div class="mb-4 flex items-center justify-between">
      <ProseH2 class="my-0 capitalize">{{ group.label }}</ProseH2>
      <span class="text-muted text-sm font-bold">{{ grouped[group.key]?.length ?? 0 }}</span>
    </div>

    <UPageGrid v-if="orientation === 'horizontal'">
      <ReuseRoadmapItem
        v-for="item in grouped[group.key]"
        :item="item"
        :badge="badges[item.status]" />
    </UPageGrid>
    <UPageList v-else>
      <ReuseRoadmapItem
        v-for="item in grouped[group.key]"
        :item="item"
        :badge="badges[item.status]" />
    </UPageList>
  </DefineRoadmapGroup>

  <UTheme :ui="theming">
    <UPageHero
      title="Roadmap"
      description="Keep up with what the Waypoints team is building next."
      :links="channels">
      <template #top>
        <div
          class="bg-primary absolute left-1/2 z-[-1] size-60 -translate-x-1/2 -translate-y-80 transform rounded-full blur-[300px] sm:size-80" />
      </template>

      <BackgroundStars />
    </UPageHero>

    <UPage>
      <UPageBody>
        <UPageSection>
          <div class="grid gap-8 lg:grid-cols-2">
            <div>
              <ReuseRoadmapGroup :group="columns['in-progress']" orientation="vertical" />
            </div>
            <div>
              <ReuseRoadmapGroup :group="columns.next" orientation="vertical" />
            </div>
          </div>
        </UPageSection>

        <UPageSection v-if="grouped.later.length">
          <ReuseRoadmapGroup :group="columns.later" orientation="horizontal" />
        </UPageSection>
      </UPageBody>
    </UPage>
  </UTheme>
</template>

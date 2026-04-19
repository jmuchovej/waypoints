<script setup lang="ts">
const { data: roadmap } = await useAsyncData("roadmap", () => queryCollection("roadmap").first());

type RoadmapItem = { title: string; description: string; status: string };

const grouped = computed(() => {
  const empty = {
    done: [] as RoadmapItem[],
    "in-progress": [] as RoadmapItem[],
    next: [] as RoadmapItem[],
    later: [] as RoadmapItem[],
  };
  if (!roadmap.value?.items) return empty;

  const groups = { ...empty };

  for (const item of roadmap.value.items) {
    groups[item.status as keyof typeof groups]?.push(item);
  }

  return groups;
});

const columns = [
  {
    key: "in-progress",
    label: "What we're currently working on",
    badge: { label: "In Progress", color: "primary" as const },
  },
  { key: "next", label: "What we're working on next", badge: null },
];

const channels = [
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
</script>

<template>
  <UPage>
    <UPageHero
      title="Roadmap"
      description="Keep up with what the Waypoints team is building next."
      :links="channels"
      class="md:border-b border-default"
      :ui="{ container: 'relative py-10 sm:py-16 lg:py-24' }"
    >
      <template #top>
        <div
          class="absolute z-[-1] rounded-full bg-primary blur-[300px] size-60 sm:size-80 transform -translate-x-1/2 left-1/2 -translate-y-80"
        />
      </template>

      <BackgroundStars />

      <div
        aria-hidden="true"
        class="hidden md:block absolute z-[-1] border-x border-default inset-0 mx-4 sm:mx-6 lg:mx-8"
      />
    </UPageHero>

    <UPageSection :ui="{ container: '!py-0' }">
      <div class="md:border-x border-default">
        <UContainer class="max-w-5xl py-12">
          <div v-if="grouped.done.length" class="mx-auto">
            <div class="flex items-center justify-between mb-4">
              <h2 class="text-lg font-semibold text-highlighted">Recently completed</h2>
              <span class="text-sm text-muted">{{ grouped.done.length }}</span>
            </div>
            <div class="grid gap-3 sm:grid-cols-2 lg:grid-cols-3">
              <UPageCard
                v-for="item in grouped.done"
                :key="item.title"
                :title="item.title"
                :description="item.description"
                spotlight
              >
                <template #trailing>
                  <UBadge color="success" variant="subtle" label="Done" />
                </template>
              </UPageCard>
            </div>
          </div>

          <div class="grid gap-8 lg:grid-cols-2">
            <div v-for="col in columns" :key="col.key">
              <div class="flex items-center justify-between mb-4">
                <h2 class="text-lg font-semibold text-highlighted">
                  {{ col.label }}
                </h2>
                <span class="text-sm text-muted">{{ grouped[col.key]?.length ?? 0 }}</span>
              </div>
              <div class="space-y-3">
                <UPageCard
                  v-for="item in grouped[col.key]"
                  :key="item.title"
                  :title="item.title"
                  :description="item.description"
                  spotlight
                >
                  <template v-if="col.badge" #trailing>
                    <UBadge v-bind="col.badge" variant="subtle" />
                  </template>
                </UPageCard>
              </div>
            </div>
          </div>
        </UContainer>

        <USeparator />

        <UContainer class="py-12 max-w-5xl" v-if="grouped.later.length">
          <div class="flex items-center justify-between mb-4">
            <h2 class="text-lg font-semibold text-highlighted">Longer term</h2>
            <span class="text-sm text-muted">{{ grouped.later.length }}</span>
          </div>
          <div class="grid gap-3 sm:grid-cols-2 lg:grid-cols-3">
            <UPageCard
              v-for="item in grouped.later"
              :key="item.title"
              :title="item.title"
              :description="item.description"
            />
          </div>
        </UContainer>
      </div>
    </UPageSection>

    <USeparator />
  </UPage>
</template>

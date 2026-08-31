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
      class="border-default md:border-b"
      :ui="{ container: 'relative py-10 sm:py-16 lg:py-24' }">
      <template #top>
        <div
          class="bg-primary absolute left-1/2 z-[-1] size-60 -translate-x-1/2 -translate-y-80 transform rounded-full blur-[300px] sm:size-80" />
      </template>

      <BackgroundStars />

      <div
        aria-hidden="true"
        class="border-default absolute inset-0 z-[-1] mx-4 hidden border-x sm:mx-6 md:block lg:mx-8" />
    </UPageHero>

    <UPageSection :ui="{ container: '!py-0' }">
      <div class="border-default md:border-x">
        <UContainer class="max-w-5xl py-12">
          <div v-if="grouped.done.length" class="mx-auto">
            <div class="mb-4 flex items-center justify-between">
              <h2 class="text-highlighted text-lg font-semibold">Recently completed</h2>
              <span class="text-muted text-sm">{{ grouped.done.length }}</span>
            </div>
            <div class="grid gap-3 sm:grid-cols-2 lg:grid-cols-3">
              <UPageCard
                v-for="item in grouped.done"
                :key="item.title"
                :title="item.title"
                :description="item.description"
                spotlight>
                <template #trailing>
                  <UBadge color="success" variant="subtle" label="Done" />
                </template>
              </UPageCard>
            </div>
          </div>

          <div class="grid gap-8 lg:grid-cols-2">
            <div v-for="col in columns" :key="col.key">
              <div class="mb-4 flex items-center justify-between">
                <h2 class="text-highlighted text-lg font-semibold">
                  {{ col.label }}
                </h2>
                <span class="text-muted text-sm">{{ grouped[col.key]?.length ?? 0 }}</span>
              </div>
              <div class="space-y-3">
                <UPageCard
                  v-for="item in grouped[col.key]"
                  :key="item.title"
                  :title="item.title"
                  :description="item.description"
                  spotlight>
                  <template v-if="col.badge" #trailing>
                    <UBadge v-bind="col.badge" variant="subtle" />
                  </template>
                </UPageCard>
              </div>
            </div>
          </div>
        </UContainer>

        <USeparator />

        <UContainer class="max-w-5xl py-12" v-if="grouped.later.length">
          <div class="mb-4 flex items-center justify-between">
            <h2 class="text-highlighted text-lg font-semibold">Longer term</h2>
            <span class="text-muted text-sm">{{ grouped.later.length }}</span>
          </div>
          <div class="grid gap-3 sm:grid-cols-2 lg:grid-cols-3">
            <UPageCard
              v-for="item in grouped.later"
              :key="item.title"
              :title="item.title"
              :description="item.description" />
          </div>
        </UContainer>
      </div>
    </UPageSection>

    <USeparator />
  </UPage>
</template>

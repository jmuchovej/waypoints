<script lang="ts" setup>
  const { title, description, headline } = defineProps<{
    title?: string;
    description?: string;
    headline?: string;
  }>();

  const appConfig = useAppConfig();
  const { name: siteName } = useSiteConfig();
  const primaryColor = appConfig.ui?.colors?.primary ?? "emerald";
</script>

<template>
  <div class="flex h-full w-full flex-col justify-between bg-neutral-950 px-[80px] py-[60px]">
    <!-- Radial glow top-right: wide soft layer -->
    <div
      class="absolute top-0 right-0 h-[700px] w-[700px] bg-[radial-gradient(circle_at_top_right,rgba(255,255,255,0.10)_0%,rgba(255,255,255,0.04)_40%,transparent_70%)]" />
    <!-- Radial glow top-right: tight bright core -->
    <div
      class="absolute top-0 right-0 h-[350px] w-[350px] bg-[radial-gradient(circle_at_top_right,rgba(255,255,255,0.22)_0%,rgba(255,255,255,0.08)_35%,transparent_65%)]" />

    <div class="flex flex-1 flex-col justify-center">
      <p
        v-if="headline"
        :class="`m-0 mb-5 text-[22px] font-bold tracking-[0.05em] uppercase text-${primaryColor}-500`">
        {{ headline }}
      </p>
      <h1
        v-if="title"
        class="m-0 mb-6 w-full max-w-[900px] text-[50px] leading-[1.1] font-bold wrap-break-word text-white">
        {{ title?.slice(0, 60) }}
      </h1>
      <p
        v-if="description"
        class="m-0 w-full max-w-[900px] text-[28px] leading-[1.4] wrap-break-word text-neutral-400">
        {{ description?.slice(0, 200) }}
      </p>
    </div>

    <div class="flex">
      <div class="rounded-lg px-5 py-2 text-[18px] font-normal text-white">
        {{ siteName }}
      </div>
    </div>
  </div>
</template>

<script lang="ts" setup>
  const { title, description } = defineProps<{
    title?: string;
    description?: string;
    headline?: string;
  }>();

  const appConfig = useAppConfig();
  const { name: siteName } = useSiteConfig();
  const primaryColor = appConfig.ui?.colors?.primary ?? "emerald";
  const logoPath = appConfig.header?.logo?.dark || appConfig.header?.logo?.light;

  const logoSvg = await fetchLogoSvg(logoPath);

  async function fetchLogoSvg(path?: string): Promise<string> {
    if (!path) return "";
    try {
      const { url: siteUrl } = useSiteConfig();
      const url = path.startsWith("http") ? path : `${siteUrl}${path}`;
      const svg = await $fetch<string>(url, { responseType: "text" });
      return svg.replace("<svg", '<svg width="48" height="48"');
    } catch {
      return "";
    }
  }
</script>

<template>
  <div class="flex h-full w-full flex-col justify-between bg-neutral-950 px-[80px] py-[60px]">
    <!-- Radial glow top-right: wide soft layer -->
    <div
      class="absolute top-0 right-0 h-[700px] w-[700px] bg-[radial-gradient(circle_at_top_right,rgba(255,255,255,0.10)_0%,rgba(255,255,255,0.04)_40%,transparent_70%)]" />
    <!-- Radial glow top-right: tight bright core -->
    <div
      class="absolute top-0 right-0 h-[350px] w-[350px] bg-[radial-gradient(circle_at_top_right,rgba(255,255,255,0.22)_0%,rgba(255,255,255,0.08)_35%,transparent_65%)]" />

    <div class="flex w-full flex-1 flex-col justify-center">
      <div v-if="logoSvg" class="mb-8 flex justify-center">
        <!-- eslint-disable-next-line vue/no-v-html -->
        <div class="h-[48px] w-[48px]" v-html="logoSvg" />
      </div>
      <div v-if="title" class="mb-6 flex justify-center">
        <h1 class="m-0 text-center text-[50px] leading-[1.1] font-bold wrap-break-word text-white">
          {{ title?.slice(0, 60) }}
        </h1>
      </div>
      <div v-if="description" class="flex justify-center">
        <p class="m-0 text-center text-[28px] leading-[1.4] wrap-break-word text-neutral-400">
          {{ description?.slice(0, 200) }}
        </p>
      </div>
    </div>

    <div class="flex">
      <div :class="`rounded-lg px-5 py-2 text-[18px] font-normal text-${primaryColor}-500`">
        {{ siteName }}
      </div>
    </div>
  </div>
</template>

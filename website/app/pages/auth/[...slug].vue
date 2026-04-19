<script setup lang="ts">
const { useSession, signIn } = useAuth();
const { data: session } = await useSession();

watch(
  () => session.value,
  (s) => {
    if (s) navigateTo("/account");
  },
  { immediate: true },
);

const form = reactive({
  email: "",
  password: "",
});

const error = ref("");
const loading = ref(false);

async function handleEmailSignIn() {
  error.value = "";
  loading.value = true;

  const { error: authError } = await signIn.email({
    email: form.email,
    password: form.password,
  });

  loading.value = false;

  if (authError) {
    error.value = authError.message ?? "Sign in failed";
  }
}

async function handleGitHubSignIn() {
  await signIn.social({ provider: "github" });
}
</script>

<template>
  <div class="flex justify-center py-16 sm:py-24">
    <div class="w-full max-w-sm space-y-6">
      <div class="text-center">
        <h1 class="text-2xl font-bold text-highlighted">Sign in to Waypoints</h1>
        <p class="text-sm text-muted mt-2">Sign in to manage your account and sync settings.</p>
      </div>

      <UButton
        block
        variant="subtle"
        color="neutral"
        icon="i-simple-icons-github"
        label="Continue with GitHub"
        @click="handleGitHubSignIn"
      />

      <USeparator label="or" />

      <form class="space-y-4" @submit.prevent="handleEmailSignIn">
        <UFormField label="Email">
          <UInput v-model="form.email" type="email" placeholder="you@example.com" required />
        </UFormField>

        <UFormField label="Password">
          <UInput v-model="form.password" type="password" placeholder="••••••••" required />
        </UFormField>

        <p v-if="error" class="text-sm text-red-500">{{ error }}</p>

        <UButton block type="submit" :loading="loading" label="Sign in" />
      </form>

      <p class="text-center text-sm text-muted">
        Don't have an account?
        <NuxtLink to="/auth/signup" class="text-primary hover:underline">Sign up</NuxtLink>
      </p>
    </div>
  </div>
</template>

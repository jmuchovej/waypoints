<script setup lang="ts">
  import * as v from "valibot";

  const { signIn, signUp } = useAuth();
  const { data: session } = await useSession();

  const route = useRoute();
  const isSignUp = computed(() => route.path.includes("signup"));

  watch(
    () => session.value,
    (s) => {
      if (s) navigateTo("/account");
    },
    { immediate: true },
  );

  const error = ref("");
  const loading = ref(false);

  const fields = computed(() => [
    ...(isSignUp.value
      ? [
          {
            name: "name",
            type: "text" as const,
            label: "Name",
            placeholder: "Your name",
            required: true,
          },
        ]
      : []),
    {
      name: "email",
      type: "email" as const,
      label: "Email",
      placeholder: "you@example.com",
      required: true,
    },
    {
      name: "password",
      type: "password" as const,
      label: "Password",
      placeholder: "••••••••",
      required: true,
    },
  ]);

  const providers = [
    {
      label: "GitHub",
      icon: "i-simple-icons-github",
      onClick: () => signIn.social({ provider: "github" }),
    },
  ];

  const schema = computed(() =>
    isSignUp.value
      ? v.object({
          name: v.pipe(v.string(), v.nonEmpty("Name is required")),
          email: v.pipe(v.string(), v.email("Invalid email")),
          password: v.pipe(v.string(), v.minLength(8, "Must be at least 8 characters")),
        })
      : v.object({
          email: v.pipe(v.string(), v.email("Invalid email")),
          password: v.pipe(v.string(), v.nonEmpty("Password is required")),
        }),
  );

  async function onSubmit(payload: { data: Record<string, string> }) {
    error.value = "";
    loading.value = true;

    if (isSignUp.value) {
      const { error: authError } = await signUp.email({
        name: payload.data.name,
        email: payload.data.email,
        password: payload.data.password,
      });
      if (authError) error.value = authError.message ?? "Sign up failed";
    } else {
      const { error: authError } = await signIn.email({
        email: payload.data.email,
        password: payload.data.password,
      });
      if (authError) error.value = authError.message ?? "Sign in failed";
    }

    loading.value = false;
  }
</script>

<template>
  <div class="flex justify-center py-16 sm:py-24">
    <UPageCard class="w-full max-w-md">
      <UAuthForm
        :title="isSignUp ? 'Create an account' : 'Sign in to Waypoints'"
        :description="
          isSignUp
            ? 'Get started with Waypoints.'
            : 'Sign in to manage your account and sync settings.'
        "
        :fields="fields"
        :providers="providers"
        :schema="schema"
        :loading="loading"
        :submit="{ label: isSignUp ? 'Sign up' : 'Sign in' }"
        @submit="onSubmit">
        <template #validation>
          <p v-if="error" class="text-sm text-red-500">{{ error }}</p>
        </template>

        <template #footer>
          <p class="text-muted text-center text-sm">
            <template v-if="isSignUp">
              Already have an account?
              <NuxtLink to="/auth/login" class="text-primary hover:underline">Sign in</NuxtLink>
            </template>
            <template v-else>
              Don't have an account?
              <NuxtLink to="/auth/signup" class="text-primary hover:underline">Sign up</NuxtLink>
            </template>
          </p>
        </template>
      </UAuthForm>
    </UPageCard>
  </div>
</template>

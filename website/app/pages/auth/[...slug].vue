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

  const signUpSchema = v.object({
    name: v.pipe(v.string(), v.nonEmpty("Name is required")),
    email: v.pipe(v.string(), v.email("Invalid email")),
    password: v.pipe(v.string(), v.minLength(8, "Must be at least 8 characters")),
  });

  const signInSchema = v.object({
    email: v.pipe(v.string(), v.email("Invalid email")),
    password: v.pipe(v.string(), v.nonEmpty("Password is required")),
  });

  const schema = computed(() => (isSignUp.value ? signUpSchema : signInSchema));

  async function onSubmit(payload: {
    data: v.InferOutput<typeof signUpSchema> | v.InferOutput<typeof signInSchema>;
  }) {
    error.value = "";
    loading.value = true;

    if ("name" in payload.data) {
      const { error: authError } = await signUp.email(payload.data);
      if (authError) error.value = authError.message ?? "Sign up failed";
    } else {
      const { error: authError } = await signIn.email(payload.data);
      if (authError) error.value = authError.message ?? "Sign in failed";
    }

    loading.value = false;
  }

  const formTitle = computed(() => (isSignUp.value ? "Create an account" : "Sign in to Waypoints"));
  const formDescription = computed(() =>
    isSignUp.value
      ? "Get started with Waypoints."
      : "Sign in to manage your account and sync settings.",
  );
</script>

<template>
  <div class="flex justify-center py-16 sm:py-24">
    <UPageCard class="w-full max-w-md">
      <UAuthForm
        :title="formTitle"
        :description="formDescription"
        :fields="fields"
        :providers="providers"
        :schema="schema"
        :loading="loading"
        :submit="{ label: isSignUp ? 'Sign up' : 'Sign in' }"
        @submit="onSubmit">
        <template #validation>
          <UAlert v-if="error" color="error" icon="i-lucide-info" :title="error" />
        </template>

        <template #footer>
          <p class="text-muted text-center text-sm">
            <template v-if="isSignUp">
              Already have an account?
              <ULink to="/auth/login" active>Sign in</ULink>
            </template>
            <template v-else>
              Don't have an account?
              <ULink to="/auth/signup" active>Sign up</ULink>
            </template>
          </p>
        </template>
      </UAuthForm>
    </UPageCard>
  </div>
</template>

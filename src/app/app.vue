<!--
  SPDX-FileCopyrightText: 2026 John Muchovej
  SPDX-License-Identifier: AGPL-3.0-or-later

  This file is part of Waypoints.

  Waypoints is free software: you can redistribute it and/or modify it
  under the terms of the GNU Affero General Public License as published
  by the Free Software Foundation, either version 3 of the License, or
  (at your option) any later version.

  Waypoints is distributed in the hope that it will be useful, but
  WITHOUT ANY WARRANTY; without even the implied warranty of
  MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the GNU
  Affero General Public License for more details.

  You should have received a copy of the GNU Affero General Public
  License along with Waypoints. If not, see <https://www.gnu.org/licenses/>.

  Additional permission under GNU AGPL version 3 section 7: this Program
  may be conveyed through application stores on the terms set out in
  docs/licensing/COPYING.exceptions.
-->

<template>
  <Html class="app-wrapper overflow-x-hidden" lang="en">
    <Body class="font-sans antialiased">
      <UApp>
        <NuxtLayout>
          <NuxtPage />
        </NuxtLayout>
      </UApp>
    </Body>
  </Html>
</template>

<style>
  @reference "~/assets/css/main.css";

  .app-wrapper {
    @apply flex h-dvh flex-col overflow-hidden;
    @apply pt-[env(safe-area-inset-top)] pb-[env(safe-area-inset-bottom)];
  }
</style>

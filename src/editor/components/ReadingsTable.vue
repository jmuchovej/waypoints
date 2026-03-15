<script lang="ts">
import type { Chart, Feature, FeatureID, Reading } from "@waypoints/domain";
type OnUpdate = (id: FeatureID, reading: Reading) => void | Promise<void>;

export interface ReadingsTableProps {
  chart: Chart;
  readings: Partial<Record<FeatureID, Reading>>;
  onUpdate: OnUpdate;
}
</script>
<script setup lang="ts">
import { registry } from "./readings";

const props = defineProps<ReadingsTableProps>();

const features = computed<[FeatureID, Feature][]>(() => {
  return Object.entries(props.chart.features ?? {})
    .filter(([, f]) => f != null)
    .map(([id, f]) => [id as FeatureID, f as Feature]);
});

function valueFor(featureId: FeatureID): Reading | undefined {
  return props.readings[featureId];
}

function handleUpdate(featureId: FeatureID, reading: Reading) {
  props.onUpdate(featureId, reading);
}
</script>

<template>
  <div v-if="features.length > 0" class="readings-table">
    <!--
      Each row is a single <div class="reading-row"> with `display: contents`
      (see style below) so its children (label + input cell) become direct
      grid children and participate in the parent's column tracks. Using a
      real wrapper element rather than <template v-for> with multiple root
      children avoids Vue's Fragment patching bug — a nested DOM update
      (e.g. inside UInputMenu) can invalidate the fragment's sibling anchor
      and crash with `parent.insertBefore(null)`.
    -->
    <div v-for="[featureId, feature] in features" :key="featureId" class="reading-row">
      <label :for="featureId" class="reading-label">
        {{ feature.name }}
        <span v-if="feature.required" class="text-error">*</span>
      </label>
      <div class="reading-input">
        <component
          :is="registry(feature)"
          :id="featureId"
          :value="valueFor(featureId)"
          :feature="feature"
          @update="(r: Reading) => handleUpdate(featureId, r)"
        />
        <p v-if="feature.description" class="reading-description">
          {{ feature.description }}
        </p>
      </div>
    </div>
  </div>
</template>

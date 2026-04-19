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
import { featureComponent } from "#imports";

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
    <div v-for="[featureId, feature] in features" :key="featureId" class="reading-row">
      <label :for="featureId" class="reading-label">
        {{ feature.name }}
        <span v-if="feature.required" class="text-error">*</span>
      </label>
      <div class="reading-input">
        <component
          :is="featureComponent(feature)"
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

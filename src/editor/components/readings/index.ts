import type { Feature } from "@waypoints/domain";
import { defineAsyncComponent, type Component } from "vue";

const TextReading = defineAsyncComponent(() => import("./TextReading.vue"));
const NumberReading = defineAsyncComponent(() => import("./NumberReading.vue"));
const CheckboxReading = defineAsyncComponent(() => import("./CheckboxReading.vue"));
const EmailReading = defineAsyncComponent(() => import("./EmailReading.vue"));
const URLReading = defineAsyncComponent(() => import("./URLReading.vue"));
const PhoneReading = defineAsyncComponent(() => import("./PhoneReading.vue"));
const DateReading = defineAsyncComponent(() => import("./DateReading.vue"));
const ReferenceReading = defineAsyncComponent(() => import("./ReferenceReading.vue"));
const FormulaReading = defineAsyncComponent(() => import("./FormulaReading.vue"));
const SelectReading = defineAsyncComponent(() => import("./SelectReading.vue"));

export const registry = (feature: Feature): Component => {
  switch (feature.kind.type) {
    case "text":
      return TextReading;
    case "number":
      return NumberReading;
    case "checkbox":
      return CheckboxReading;
    case "email":
      return EmailReading;
    case "url":
      return URLReading;
    case "phone":
      return PhoneReading;
    case "date":
      return DateReading;
    case "reference":
      return ReferenceReading;
    case "formula":
      return FormulaReading;
    case "select":
    case "multi-select":
    case "grouped-select":
    case "grouped-multi-select":
      return SelectReading;
  }
};

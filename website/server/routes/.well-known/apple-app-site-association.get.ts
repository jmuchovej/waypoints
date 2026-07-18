// Apple Universal Links association: tells iOS/macOS which app IDs may
// handle https://waypoints.so/pair links. Served as a route (not a static
// asset) so the team ID rides runtimeConfig and per-environment conditions
// stay possible. Apple fetches this path extensionless and expects JSON —
// Nitro's object serialization sets the content type.
//
// Note: only the SSR/Worker deploy serves this; the static-generate escape
// hatch drops server routes, which is fine — link association only matters
// on the production domain.

const BUNDLE_IDS = ["so.waypoints", "so.waypoints.preview", "so.waypoints.dev"];

export default defineEventHandler((event) => {
  const { appleTeamId } = useRuntimeConfig(event).appAssociation;
  return {
    applinks: {
      details: [
        {
          appIDs: BUNDLE_IDS.map((id) => `${appleTeamId}.${id}`),
          components: [{ "/": "/pair" }],
        },
      ],
    },
  };
});

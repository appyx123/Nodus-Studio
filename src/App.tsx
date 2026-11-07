import { createResource } from "solid-js";
import Workbench from "./components/canvas/Workbench";
import { loadCatalog } from "./components/library/ComponentCatalog";

export default function App() {
  const [catalog, { refetch }] = createResource(loadCatalog);

  return (
    <div class="app-shell h-screen w-screen overflow-hidden bg-zinc-950 text-zinc-100 flex flex-col">
      <h1 class="sr-only">Nodus Studio</h1>
      <Workbench
        catalog={catalog}
        onReload={() => {
          void refetch();
        }}
      />
    </div>
  );
}

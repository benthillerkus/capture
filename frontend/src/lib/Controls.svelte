<script>
  const API_HOST = import.meta.env.VITE_API_HOST;

  let x = $state(0);
  let y = $state(0);
  let multiview_mode = $state("none");
  let anaglyph_format = $state("red-cyan");

  $effect(() => {
    fetch(`${API_HOST}/api/configuration`, {
      method: "POST",
      headers: {
        "Content-Type": "application/json",
      },
      body: JSON.stringify({
        convergence: [x ?? 0, y ?? 0],
        multiview_mode,
        anaglyph_format,
      }),
    });
  });
</script>

<div>
  <label>
    X
    <input bind:value={x} type="number" />
  </label>
  <label>
    Y
    <input bind:value={y} type="number" />
  </label>
  <label>
    Multiview Mode
    <select bind:value={multiview_mode}>
      <option value="left">Left</option>
      <option value="right">Right</option>
      <option value="top-bottom">Top-Bottom</option>
      <option value="side-by-side">Side-by-Side</option>
      <option value="mono">Anaglyph</option>
      <option value="none">None</option>
      <option value="checkerboard">Checkerboard</option>
      <option value="column-interleaved">Column Interleaved</option>
      <option value="row-interleaved">Row Interleaved</option>
      <option value="frame-by-frame">Frame-by-Frame</option>
    </select>
  </label>
  {#if multiview_mode === "mono"}
    <label>
      <select bind:value={anaglyph_format}>
        <option value="red-cyan">Red-Cyan</option>
        <option value="green-magenta">Green-Magenta</option>
        <option value="amber-blue">Amber-Blue</option>
      </select>
    </label>
  {/if}
</div>

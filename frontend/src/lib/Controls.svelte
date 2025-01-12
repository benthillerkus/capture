<script>
  const API_HOST = import.meta.env.VITE_API_HOST;

  let x = $state(0);
  let y = $state(0);
  let multiview_mode = $state("none");
  let anaglyph_format = $state("red-cyan");
  let resolution_idx = $state("4");
  let codec = $state("MotionJpeg");

  $effect(() => {
    let _ = resolution_idx;
    console.log(resolution_idx);
    let resolution = (() => {
      switch (resolution_idx) {
        case "0":
          return { width: 3280, height: 2464, fps: 21 };
        case "1":
          return { width: 3820, height: 1848, fps: 28 };
        case "2":
          return { width: 1920, height: 1080, fps: 30 };
        case "3":
          return { width: 1640, height: 1232, fps: 30 };
        default:
          return { width: 1280, height: 720, fps: 60 };
      }
    })();

    fetch(`${API_HOST}/api/configuration`, {
      method: "POST",
      headers: {
        "Content-Type": "application/json",
      },
      body: JSON.stringify({
        convergence: [x ?? 0, y ?? 0],
        multiview_mode,
        anaglyph_format,
        ...resolution,
        codec,
      }),
    })
      .then((response) => {
        return response.json();
      })
      .then((body) => {
        x = body.convergence[0];
        y = body.convergence[1];
        multiview_mode = body.multiview_mode;
        anaglyph_format = body.anaglyph_format;
        switch (body.height) {
          case 2464:
            resolution_idx = "0";
            break;
          case 1848:
            resolution_idx = "1";
            break;
          case 1080:
            resolution_idx = "2";
            break;
          case 1232:
            resolution_idx = "3";
            break;
          case 720:
            resolution_idx = "4";
            break;
        }
        codec = body.codec;
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
      <option selected value="mono">Anaglyph</option>
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
  <label>
    Resolution
    <select bind:value={resolution_idx}>
      <option value="0">3280x2464@21fps</option>
      <option value="1">3820x1848@28fps</option>
      <option value="2">1920x1080@30fps</option>
      <option value="3">1640x1232@30fps</option>
      <option value="4"> 1280x720@60fps</option>
    </select>
  </label>
  <label
    >Codec
    <select bind:value={codec}>
      <option value="Prores"></option>
      <option value="MotionJpeg"></option>
    </select>
  </label>
</div>

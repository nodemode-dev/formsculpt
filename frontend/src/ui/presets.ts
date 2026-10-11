export interface SkeletonPreset {
  spheres: [number, number, number, number][];
  links: [number, number][];
}

function biped(): SkeletonPreset {
  return {
    spheres: [
      [0, 1.6, 0, 0.22], [0, 1.25, 0, 0.12], [0, 1.0, 0, 0.3], [0, 0.5, 0, 0.26],
      [0.4, 1.15, 0, 0.1], [0.75, 0.85, 0, 0.07], [1.0, 0.6, 0, 0.06],
      [-0.4, 1.15, 0, 0.1], [-0.75, 0.85, 0, 0.07], [-1.0, 0.6, 0, 0.06],
      [0.18, 0.25, 0, 0.12], [0.2, -0.3, 0, 0.09], [0.2, -0.85, 0.05, 0.08],
      [-0.18, 0.25, 0, 0.12], [-0.2, -0.3, 0, 0.09], [-0.2, -0.85, 0.05, 0.08],
    ],
    links: [[0, 1], [1, 2], [2, 3], [2, 4], [4, 5], [5, 6], [2, 7], [7, 8], [8, 9], [3, 10], [10, 11], [11, 12], [3, 13], [13, 14], [14, 15]],
  };
}

function quadruped(): SkeletonPreset {
  return {
    spheres: [
      [-0.7, 0.8, 0, 0.28], [0, 0.85, 0, 0.32], [0.7, 0.85, 0, 0.28], [1.0, 1.15, 0, 0.14], [1.25, 1.35, 0, 0.2], [1.5, 1.25, 0, 0.1],
      [0.75, 0.4, 0.22, 0.09], [0.78, 0, 0.22, 0.08], [0.75, 0.4, -0.22, 0.09], [0.78, 0, -0.22, 0.08],
      [-0.75, 0.4, 0.22, 0.1], [-0.72, 0, 0.22, 0.08], [-0.75, 0.4, -0.22, 0.1], [-0.72, 0, -0.22, 0.08],
      [-1.0, 0.95, 0, 0.08], [-1.4, 1.1, 0, 0.04],
    ],
    links: [[0, 1], [1, 2], [2, 3], [3, 4], [4, 5], [2, 6], [6, 7], [2, 8], [8, 9], [0, 10], [10, 11], [0, 12], [12, 13], [0, 14], [14, 15]],
  };
}

function hand(): SkeletonPreset {
  const spheres: SkeletonPreset['spheres'] = [[-0.2, 0.1, 0, 0.2], [0.2, 0.1, 0, 0.2], [-0.2, 0.4, 0, 0.2], [0.2, 0.4, 0, 0.2]];
  const links: SkeletonPreset['links'] = [[0, 1], [0, 2], [1, 3], [2, 3]];
  const fingers: [number, number][] = [[-0.27, 0.85], [-0.09, 1.0], [0.09, 0.95], [0.27, 0.8]];
  for (const [x, len] of fingers) {
    let prev = x < 0 ? 2 : 3;
    for (let j = 0; j < 4; j++) {
      spheres.push([x, 0.55 + (len - 0.15) * (j / 3) * 1.1, 0, 0.09 - j * 0.01]);
      links.push([prev, spheres.length - 1]);
      prev = spheres.length - 1;
    }
  }
  let prev = 1;
  for (const s of [[0.35, 0.1, 0.05, 0.1], [0.55, 0.25, 0.08, 0.09], [0.7, 0.45, 0.1, 0.08], [0.8, 0.6, 0.1, 0.07]] as const) {
    spheres.push([s[0], s[1], s[2], s[3]]);
    links.push([prev, spheres.length - 1]);
    prev = spheres.length - 1;
  }
  return { spheres, links };
}

function snake(): SkeletonPreset {
  const spheres: SkeletonPreset['spheres'] = [];
  const links: SkeletonPreset['links'] = [];
  const n = 12;
  for (let i = 0; i < n; i++) {
    const t = i / (n - 1);
    spheres.push([1.4 - t * 2.8, 0.15, Math.sin(t * Math.PI * 2.2) * 0.35, i === 0 ? 0.2 : 0.17 - t * 0.13]);
    if (i > 0) links.push([i - 1, i]);
  }
  return { spheres, links };
}

export const SKELETONS: Record<string, { label: string; build: () => SkeletonPreset }> = {
  biped: { label: 'Biped', build: biped },
  quadruped: { label: 'Quadruped', build: quadruped },
  hand: { label: 'Hand', build: hand },
  snake: { label: 'Snake', build: snake },
};

export const LATHES: Record<string, { label: string; profile: [number, number][] }> = {
  vase: { label: 'Vase', profile: [[0, 0], [0.35, 0], [0.5, 0.3], [0.45, 0.7], [0.22, 1.0], [0.25, 1.2], [0.32, 1.3], [0, 1.3]] },
  bottle: { label: 'Bottle', profile: [[0, 0], [0.3, 0], [0.32, 0.05], [0.32, 0.8], [0.25, 0.95], [0.1, 1.05], [0.09, 1.3], [0.11, 1.32], [0.11, 1.36], [0, 1.36]] },
  goblet: { label: 'Goblet', profile: [[0, 0], [0.3, 0], [0.3, 0.03], [0.06, 0.1], [0.05, 0.5], [0.3, 0.7], [0.38, 1.0], [0, 1.0]] },
  column: { label: 'Column', profile: [[0, 0], [0.4, 0], [0.4, 0.12], [0.3, 0.15], [0.28, 0.2], [0.25, 1.8], [0.28, 1.85], [0.4, 1.88], [0.4, 2.0], [0, 2.0]] },
};

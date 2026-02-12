declare module 'react-force-graph-2d' {
  import { Component, RefObject } from 'react';

  export interface GraphData {
    nodes: NodeObject[];
    links: LinkObject[];
  }

  export interface NodeObject {
    id: string | number;
    x?: number;
    y?: number;
    vx?: number;
    vy?: number;
    fx?: number;
    fy?: number;
    [key: string]: unknown;
  }

  export interface LinkObject {
    source: string | number | NodeObject;
    target: string | number | NodeObject;
    [key: string]: unknown;
  }

  export interface ForceGraphProps {
    graphData?: GraphData;
    width?: number;
    height?: number;
    backgroundColor?: string;
    nodeLabel?: string | ((node: NodeObject) => string);
    nodeColor?: string | ((node: NodeObject) => string);
    nodeVal?: number | ((node: NodeObject) => number);
    nodeRelSize?: number;
    nodeCanvasObject?: (node: NodeObject, ctx: CanvasRenderingContext2D, globalScale: number) => void;
    nodeCanvasObjectMode?: string | ((node: NodeObject) => string);
    linkSource?: string;
    linkTarget?: string;
    linkLabel?: string | ((link: LinkObject) => string);
    linkColor?: string | ((link: LinkObject) => string);
    linkWidth?: number | ((link: LinkObject) => number);
    linkDirectionalArrowLength?: number | ((link: LinkObject) => number);
    linkDirectionalArrowRelPos?: number | ((link: LinkObject) => number);
    onNodeClick?: (node: NodeObject, event: MouseEvent) => void;
    onNodeRightClick?: (node: NodeObject, event: MouseEvent) => void;
    onNodeHover?: (node: NodeObject | null, previousNode: NodeObject | null) => void;
    onLinkClick?: (link: LinkObject, event: MouseEvent) => void;
    onLinkHover?: (link: LinkObject | null, previousLink: LinkObject | null) => void;
    onBackgroundClick?: (event: MouseEvent) => void;
    cooldownTicks?: number;
    cooldownTime?: number;
    onEngineStop?: () => void;
    ref?: RefObject<ForceGraphMethods>;
    [key: string]: unknown;
  }

  export interface ForceGraphMethods {
    zoomToFit: (duration?: number, padding?: number) => void;
    centerAt: (x?: number, y?: number, duration?: number) => void;
    zoom: (scale?: number, duration?: number) => void;
    d3Force: (forceName: string, force?: unknown) => unknown;
    d3ReheatSimulation: () => void;
    emitParticle: (link: LinkObject) => void;
  }

  const ForceGraph2D: React.FC<ForceGraphProps>;
  export default ForceGraph2D;
}

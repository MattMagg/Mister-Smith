import React, { useEffect, useRef, useState } from 'react';
import * as d3 from 'd3';
import { Discovery, DISCOVERY_COLORS } from '../../types/discovery';

interface Node extends d3.SimulationNodeDatum {
  id: string;
  discovery: Discovery;
}

interface Link extends d3.SimulationLinkDatum<Node> {
  source: string | Node;
  target: string | Node;
}

interface DiscoveryFlowGraphProps {
  discoveries: Discovery[];
  width?: number;
  height?: number;
  className?: string;
}

export const DiscoveryFlowGraph: React.FC<DiscoveryFlowGraphProps> = ({
  discoveries,
  width = 800,
  height = 600,
  className,
}) => {
  const svgRef = useRef<SVGSVGElement>(null);
  const [tooltip, setTooltip] = useState<{ x: number; y: number; discovery: Discovery } | null>(null);
  
  useEffect(() => {
    if (!svgRef.current || discoveries.length === 0) return;
    
    // Clear previous graph
    d3.select(svgRef.current).selectAll('*').remove();
    
    // Prepare nodes and links
    const nodes: Node[] = discoveries.map(d => ({
      id: d.id,
      discovery: d,
    }));
    
    const links: Link[] = discoveries
      .filter(d => d.relatedTo)
      .map(d => ({
        source: d.relatedTo!,
        target: d.id,
      }));
    
    // Create SVG
    const svg = d3.select(svgRef.current)
      .attr('viewBox', `0 0 ${width} ${height}`)
      .classed('d3-svg', true);
    
    // Create zoom behavior
    const zoom = d3.zoom<SVGSVGElement, unknown>()
      .scaleExtent([0.5, 4])
      .on('zoom', (event) => {
        g.attr('transform', event.transform);
      });
    
    svg.call(zoom);
    
    // Create main group
    const g = svg.append('g');
    
    // Create force simulation
    const simulation = d3.forceSimulation<Node>(nodes)
      .force('link', d3.forceLink<Node, Link>(links).id(d => d.id).distance(100))
      .force('charge', d3.forceManyBody().strength(-300))
      .force('center', d3.forceCenter(width / 2, height / 2))
      .force('collision', d3.forceCollide().radius(30));
    
    // Create links
    const link = g.append('g')
      .selectAll('line')
      .data(links)
      .join('line')
      .attr('stroke', '#4B5563')
      .attr('stroke-opacity', 0.6)
      .attr('stroke-width', 2);
    
    // Create nodes
    const node = g.append('g')
      .selectAll('g')
      .data(nodes)
      .join('g')
      .call(d3.drag<SVGGElement, Node>()
        .on('start', dragStarted)
        .on('drag', dragged)
        .on('end', dragEnded)
      );
    
    // Add circles to nodes
    node.append('circle')
      .attr('r', d => 20 + d.discovery.confidence * 10)
      .attr('fill', d => DISCOVERY_COLORS[d.discovery.type])
      .attr('fill-opacity', d => 0.3 + d.discovery.confidence * 0.7)
      .attr('stroke', d => DISCOVERY_COLORS[d.discovery.type])
      .attr('stroke-width', 2);
    
    // Add type icons
    node.append('text')
      .text(d => d.discovery.type[0].toUpperCase())
      .attr('text-anchor', 'middle')
      .attr('dominant-baseline', 'central')
      .attr('fill', 'white')
      .attr('font-size', '12px')
      .attr('font-weight', 'bold');
    
    // Add agent labels
    node.append('text')
      .text(d => d.discovery.agentId)
      .attr('text-anchor', 'middle')
      .attr('y', 35)
      .attr('fill', '#9CA3AF')
      .attr('font-size', '10px');
    
    // Add hover effects
    node.on('mouseenter', (event, d) => {
      const [x, y] = d3.pointer(event, svgRef.current);
      setTooltip({ x, y, discovery: d.discovery });
    })
    .on('mouseleave', () => {
      setTooltip(null);
    });
    
    // Update positions on tick
    simulation.on('tick', () => {
      link
        .attr('x1', d => (d.source as Node).x!)
        .attr('y1', d => (d.source as Node).y!)
        .attr('x2', d => (d.target as Node).x!)
        .attr('y2', d => (d.target as Node).y!);
      
      node.attr('transform', d => `translate(${d.x},${d.y})`);
    });
    
    // Drag functions
    function dragStarted(event: d3.D3DragEvent<SVGGElement, Node, Node>) {
      if (!event.active) simulation.alphaTarget(0.3).restart();
      event.subject.fx = event.subject.x;
      event.subject.fy = event.subject.y;
    }
    
    function dragged(event: d3.D3DragEvent<SVGGElement, Node, Node>) {
      event.subject.fx = event.x;
      event.subject.fy = event.y;
    }
    
    function dragEnded(event: d3.D3DragEvent<SVGGElement, Node, Node>) {
      if (!event.active) simulation.alphaTarget(0);
      event.subject.fx = null;
      event.subject.fy = null;
    }
    
    // Cleanup
    return () => {
      simulation.stop();
    };
  }, [discoveries, width, height]);
  
  return (
    <div className={className} style={{ position: 'relative' }}>
      <svg ref={svgRef} className="w-full h-full bg-gray-900 rounded-lg" />
      
      {tooltip && (
        <div
          className="d3-tooltip"
          style={{
            left: tooltip.x + 10,
            top: tooltip.y - 10,
          }}
        >
          <div className="font-semibold mb-1">{tooltip.discovery.type}</div>
          <div className="text-xs">{tooltip.discovery.content}</div>
          <div className="text-xs mt-1">
            Confidence: {Math.round(tooltip.discovery.confidence * 100)}%
          </div>
        </div>
      )}
    </div>
  );
};
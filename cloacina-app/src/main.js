function displayWorkflowVisualization(graphData, layout = "hierarchical") {
  console.log("Displaying graph with data:", graphData);

  // Clear previous visualization
  d3.select("#workflow-svg").selectAll("*").remove();

  if (!graphData.nodes || graphData.nodes.length === 0) {
    d3.select("#workflow-svg")
      .append("text")
      .attr("x", "50%")
      .attr("y", "50%")
      .attr("text-anchor", "middle")
      .attr("fill", "var(--text-secondary)")
      .text("No tasks found in this workflow");
    return;
  }

  const svg = d3.select("#workflow-svg");
  const container = document.querySelector("#graph-container");
  const width = container.clientWidth;
  const height = 600;

  svg.attr("width", width).attr("height", height);

  // Create a new directed graph
  const g = new dagreD3.graphlib.Graph()
    .setGraph({
      nodesep: 50,    // Horizontal separation between nodes
      ranksep: 60,    // Vertical separation between ranks
      rankdir: "LR",  // Left-to-right layout
      marginx: 20,
      marginy: 20
    })
    .setDefaultEdgeLabel(() => ({}));

  // Add nodes to the graph
  graphData.nodes.forEach(node => {
    const nodeHtml = `
      <div class="dagre-node" style="
        background: var(--primary);
        color: white;
        border-radius: 8px;
        padding: 8px 12px;
        text-align: center;
        min-width: 80px;
        box-shadow: 0 2px 4px rgba(0,0,0,0.1);
      ">
        <div style="font-weight: 500; font-size: 11px;">${node.label}</div>
        ${document.querySelector("#show-details").checked && node.description ?
          `<div style="font-size: 9px; opacity: 0.8; margin-top: 2px;">${node.description.length > 30 ? node.description.substring(0, 30) + '...' : node.description}</div>` :
          ''}
      </div>
    `;

    g.setNode(node.id, {
      labelType: "html",
      label: nodeHtml,
      width: 100,
      height: document.querySelector("#show-details").checked && node.description ? 50 : 35,
      data: node // Store original node data for click events
    });
  });

  // Add edges to the graph
  graphData.edges.forEach(edge => {
    g.setEdge(edge.source, edge.target, {
      style: "stroke: var(--text-secondary); stroke-width: 2px; fill: none;",
      arrowheadStyle: "fill: var(--text-secondary);"
    });
  });

  // Create the renderer
  const render = new dagreD3.render();

  // Create SVG group for the graph
  const svgGroup = svg.append("g");

  // Run the renderer
  render(svgGroup, g);

  // Add click handlers to nodes
  svgGroup.selectAll(".node")
    .on("click", function(event, nodeId) {
      const nodeData = g.node(nodeId);
      if (nodeData && nodeData.data) {
        showTaskDetails(event, nodeData.data);
      }
    })
    .on("mouseover", function(event, nodeId) {
      // Highlight connected edges
      const nodeData = g.node(nodeId);
      if (nodeData && nodeData.data) {
        highlightNodeConnections(nodeId);
      }
    })
    .on("mouseout", function() {
      // Remove highlights
      unhighlightNode();
    });

  // Center the graph
  const graphBounds = svgGroup.node().getBBox();
  const xCenterOffset = (width - graphBounds.width) / 2;
  const yCenterOffset = (height - graphBounds.height) / 2;

  svgGroup.attr("transform", `translate(${xCenterOffset}, ${yCenterOffset})`);

  // Add zoom and pan
  currentZoom = d3.zoom()
    .scaleExtent([0.1, 3])
    .on("zoom", (event) => {
      svgGroup.attr("transform", event.transform);
    });

  svg.call(currentZoom);

  // Set initial zoom to fit content
  const padding = 40;
  const scale = Math.min(
    (width - padding) / graphBounds.width,
    (height - padding) / graphBounds.height,
    1 // Don't zoom in beyond 100%
  );

  const centerX = width / 2;
  const centerY = height / 2;
  const translateX = centerX - (graphBounds.width * scale) / 2;
  const translateY = centerY - (graphBounds.height * scale) / 2;

  svg.call(
    currentZoom.transform,
    d3.zoomIdentity.translate(translateX, translateY).scale(scale)
  );

  // Store current graph reference
  currentSimulation = null; // Not using D3 simulation anymore
  window.currentDagreGraph = g; // Store for debugging
}

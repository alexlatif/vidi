export class Dash {
    private wasm: any;
    
    constructor() {
        import('./dash_wasm.js').then(module => {
            this.wasm = module;
            this.wasm.JsDash.new();
        });
    }
    
    async init() {
        await import('./dash_wasm.js');
        this.wasm = window.dash_wasm;
        new this.wasm.JsDash();
    }
    
    plot2D(title: string, row: number, col: number) {
        return this.wasm.JsDash.prototype.addPlot2D(title, row, col);
    }
    
    addLine(plotId: number, name: string, color: [number, number, number], x: number[], y: number[]) {
        const points = new Float32Array(x.length * 2);
        for (let i = 0; i < x.length; i++) {
            points[i * 2] = x[i];
            points[i * 2 + 1] = y[i];
        }
        
        this.wasm.JsDash.prototype.addTrace(plotId, name, ...color, points);
    }
    
    updateLine(plotId: number, traceIdx: number, x: number[], y: number[]) {
        const points = new Float32Array(x.length * 2);
        for (let i = 0; i < x.length; i++) {
            points[i * 2] = x[i];
            points[i * 2 + 1] = y[i];
        }
        
        this.wasm.JsDash.prototype.updateTrace(plotId, traceIdx, points);
    }
    
    show(canvasId: string) {
        this.wasm.JsDash.prototype.show(canvasId);
    }
}

// Convenience functions
export function createDashboard(grid: [number, number], background: [number, number, number]) {
    const dash = new Dash();
    dash.wasm.JsDash.prototype.setGrid(...grid);
    dash.wasm.JsDash.prototype.setBackground(...background);
    return dash;
}

export function realtimePlot(canvasId: string, plots: Array<{
    title: string;
    position: [number, number];
    traces: Array<{
        name: string;
        color: [number, number, number];
        data: () => { x: number[], y: number[] };
    }>;
}>) {
    const dash = new Dash();
    
    plots.forEach((plot, i) => {
        const plotId = dash.plot2D(plot.title, ...plot.position);
        
        plot.traces.forEach((trace, j) => {
            const initial = trace.data();
            dash.addLine(plotId, trace.name, trace.color, initial.x, initial.y);
        });
    });
    
    dash.show(canvasId);
    
    // Update loop
    return {
        update: () => {
            plots.forEach((plot, i) => {
                plot.traces.forEach((trace, j) => {
                    const data = trace.data();
                    dash.updateLine(i, j, data.x, data.y);
                });
            });
        }
    };
}
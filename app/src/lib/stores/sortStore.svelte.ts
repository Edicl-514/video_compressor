type SortColumn = "name" | "size" | "resolution" | "bitrate" | "encoder";
type SortDirection = "asc" | "desc";

interface SortState {
    column: SortColumn | null;
    direction: SortDirection;
}

class SortStore {
    private state = $state<SortState>({
        column: null,
        direction: "asc"
    });

    get value() {
        return this.state;
    }

    get column() {
        return this.state.column;
    }

    get direction() {
        return this.state.direction;
    }

    setColumn(column: SortColumn | null) {
        this.state.column = column;
    }

    setDirection(direction: SortDirection) {
        this.state.direction = direction;
    }

    toggleSort(column: SortColumn) {
        if (this.state.column === column) {
            this.state.direction = this.state.direction === "asc" ? "desc" : "asc";
        } else {
            this.state.column = column;
            this.state.direction = "asc";
        }
    }

    reset() {
        this.state.column = null;
        this.state.direction = "asc";
    }
}

export const sortStore = new SortStore();
export type { SortColumn, SortDirection };

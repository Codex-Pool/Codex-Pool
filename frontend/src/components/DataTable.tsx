import {
  type Key,
  type ReactNode,
  useEffect,
  useId,
  useMemo,
  useState,
} from "react"
import {
  type ColumnDef,
  type PaginationState,
  type SortingState,
  flexRender,
  getCoreRowModel,
  getPaginationRowModel,
  getSortedRowModel,
  useReactTable,
} from "@tanstack/react-table"
import {
  Button,
  Dropdown,
  DropdownItem,
  DropdownMenu,
  DropdownTrigger,
  Input,
  Pagination,
  Select,
  SelectItem,
  Spinner,
  Table,
  TableBody,
  TableCell,
  TableColumn,
  TableHeader,
  TableRow,
  type Selection,
  type SortDescriptor,
} from "@heroui/react"
import {
  ArrowDown,
  ArrowUp,
  ArrowUpDown,
  ChevronDown,
  ChevronsLeft,
  ChevronLeft,
  ChevronRight,
  ChevronsRight,
  Inbox,
  MoreVertical,
  Plus,
  Search,
  X,
} from "lucide-react"
import { useTranslation } from "react-i18next"

import { cn } from "@/lib/utils"
import { Checkbox } from "@/components/ui/checkbox"
import { EmptyState } from "@/components/ui/empty-state"

export interface DataTableColumn<T> {
  uid: string
  name: string
  sortable?: boolean
  render?: (item: T, columnKey: string) => ReactNode
  hidden?: boolean
  width?: number
}

export interface StatusOption {
  uid: string
  name: string
}

export interface BatchActionItem {
  key: string
  label: string
  icon?: ReactNode
  color?: "default" | "danger"
}

type SharedDataTableProps<T, TValue> = {
  columns: DataTableColumn<T>[] | ColumnDef<T, TValue>[]
  data: T[]
  className?: string
  rowClassName?: string | ((row: T) => string)
  density?: "comfortable" | "compact"
  filters?: ReactNode
  actions?: ReactNode
  defaultPageSize?: number
  defaultRowsPerPage?: number
  pageSizeOptions?: number[]
  searchPlaceholder?: string
  searchField?: string
  searchFn?: (row: T, keyword: string) => boolean
  enableSearch?: boolean
  emptyText?: string
  onFilteredDataChange?: (rows: T[]) => void
  statusOptions?: StatusOption[]
  statusField?: string
  primaryActionText?: string
  onPrimaryAction?: () => void
  rowActions?: { key: string; label: string; color?: "default" | "danger" }[]
  onRowAction?: (actionKey: string, item: T) => void
  defaultSortDescriptor?: SortDescriptor
  extraTopContent?: ReactNode
  title?: string
  subtitle?: string
  isLoading?: boolean
  showToolbar?: boolean
  showPageControls?: boolean
  enableRowSelection?: boolean
  getRowId?: (row: T) => string
  selectedKeys?: Set<string>
  onSelectionChange?: (keys: Set<string>) => void
  batchActions?: BatchActionItem[]
  onBatchAction?: (actionKey: string, selectedIds: string[]) => void
}

export type DataTableProps<T, TValue = unknown> = SharedDataTableProps<T, TValue>

const DEFAULT_PAGE_SIZE_OPTIONS = [10, 20, 50, 100]

function isSimpleColumn<T, TValue>(
  column: DataTableColumn<T> | ColumnDef<T, TValue>,
): column is DataTableColumn<T> {
  return (
    typeof column === "object"
    && column !== null
    && "uid" in column
    && "name" in column
  )
}

function defaultSearchFn<T>(row: T, keyword: string) {
  try {
    return JSON.stringify(row).toLowerCase().includes(keyword)
  } catch {
    return false
  }
}

function sortIndicator(sorted: false | "asc" | "desc") {
  if (sorted === "asc") {
    return <ArrowUp className="h-3.5 w-3.5" aria-hidden="true" />
  }
  if (sorted === "desc") {
    return <ArrowDown className="h-3.5 w-3.5" aria-hidden="true" />
  }
  return <ArrowUpDown className="h-3.5 w-3.5 opacity-45" aria-hidden="true" />
}

function normalizePageSelection(selection: Selection) {
  if (selection === "all") {
    return ""
  }

  const [first] = Array.from(selection)
  return first === undefined ? "" : String(first)
}

function capitalizeLabel(value: string) {
  return value.charAt(0).toUpperCase() + value.slice(1)
}

export function DataTable<T, TValue = unknown>({
  columns,
  data,
  className,
  rowClassName,
  density = "comfortable",
  filters,
  actions,
  defaultPageSize,
  defaultRowsPerPage = 20,
  pageSizeOptions = DEFAULT_PAGE_SIZE_OPTIONS,
  searchPlaceholder,
  searchField,
  searchFn,
  enableSearch = true,
  emptyText,
  onFilteredDataChange,
  statusOptions,
  statusField,
  primaryActionText,
  onPrimaryAction,
  rowActions,
  onRowAction,
  defaultSortDescriptor,
  extraTopContent,
  title,
  subtitle,
  isLoading = false,
  showToolbar = true,
  showPageControls = true,
  enableRowSelection = false,
  getRowId,
  selectedKeys: controlledSelectedKeys,
  onSelectionChange,
  batchActions,
  onBatchAction,
}: DataTableProps<T, TValue>) {
  const { t } = useTranslation()
  const resolvedDefaultPageSize = defaultPageSize ?? defaultRowsPerPage
  const [keyword, setKeyword] = useState("")
  const [statusFilter, setStatusFilter] = useState<Selection>("all")
  const [sorting, setSorting] = useState<SortingState>(() =>
    defaultSortDescriptor?.column
      ? [
          {
            id: String(defaultSortDescriptor.column),
            desc: defaultSortDescriptor.direction === "descending",
          },
        ]
      : [],
  )
  const [pagination, setPagination] = useState<PaginationState>({
    pageIndex: 0,
    pageSize: resolvedDefaultPageSize,
  })
  const searchInputId = useId()

  // ── Selection state ──
  const [internalSelectedKeys, setInternalSelectedKeys] = useState<Set<string>>(new Set())
  const resolvedSelectedKeys = controlledSelectedKeys ?? internalSelectedKeys
  const setSelectedKeys = (next: Set<string>) => {
    if (controlledSelectedKeys === undefined) setInternalSelectedKeys(next)
    onSelectionChange?.(next)
  }
  const resolvedGetRowId = useMemo<(row: T, index: number) => string>(() => {
    if (getRowId) return (row) => getRowId(row)
    return (row, index) => {
      const r = row as Record<string, unknown>
      return r.id != null ? String(r.id) : String(index)
    }
  }, [getRowId])
  const selectionCount = resolvedSelectedKeys.size

  const simpleMode = columns.length > 0 && isSimpleColumn(columns[0])
  const simpleColumns = useMemo(
    () => (simpleMode ? (columns as DataTableColumn<T>[]) : []),
    [columns, simpleMode],
  )
  const advancedColumns = useMemo(
    () => (simpleMode ? [] : (columns as ColumnDef<T, TValue>[])),
    [columns, simpleMode],
  )
  const [visibleColumnIds, setVisibleColumnIds] = useState<Selection>(() => {
    if (!simpleMode) {
      return "all"
    }
    return new Set(
      simpleColumns
        .filter((column) => !column.hidden)
        .map((column) => column.uid),
    )
  })

  const matcher = useMemo(() => {
    if (searchFn) {
      return searchFn
    }
    if (searchField) {
      return (row: T, query: string) =>
        String((row as Record<string, unknown>)[searchField] ?? "")
          .toLowerCase()
          .includes(query)
    }
    return defaultSearchFn<T>
  }, [searchField, searchFn])

  const normalizedKeyword = keyword.trim().toLowerCase()

  const filteredData = useMemo(() => {
    let resolved = data

    if (enableSearch && normalizedKeyword) {
      resolved = resolved.filter((item) => matcher(item, normalizedKeyword))
    }

    if (statusFilter !== "all" && statusField && (statusFilter as Set<Key>).size > 0) {
      resolved = resolved.filter((item) =>
        (statusFilter as Set<Key>).has(String((item as Record<string, unknown>)[statusField] ?? "")),
      )
    }

    return resolved
  }, [data, enableSearch, matcher, normalizedKeyword, statusField, statusFilter])

  useEffect(() => {
    onFilteredDataChange?.(filteredData)
  }, [filteredData, onFilteredDataChange])

  const filteredRowIds = useMemo(
    () => filteredData.map((row, index) => resolvedGetRowId(row, index)),
    [filteredData, resolvedGetRowId],
  )

  useEffect(() => {
    if (!enableRowSelection) {
      return
    }

    const validIds = new Set(filteredRowIds)
    const cleaned = new Set([...resolvedSelectedKeys].filter((id) => validIds.has(id)))
    if (cleaned.size === resolvedSelectedKeys.size) {
      return
    }
    setSelectedKeys(cleaned)
  }, [enableRowSelection, filteredRowIds, resolvedSelectedKeys])

  useEffect(() => {
    setPagination((prev) => ({ ...prev, pageIndex: 0 }))
  }, [normalizedKeyword, statusFilter])

  const normalizedPageSizes = useMemo(() => {
    const merged = [...pageSizeOptions, resolvedDefaultPageSize]
    return Array.from(new Set(merged)).sort((a, b) => a - b)
  }, [pageSizeOptions, resolvedDefaultPageSize])

  const activeSimpleColumns = useMemo(() => {
    if (!simpleMode) {
      return []
    }
    if (visibleColumnIds === "all") {
      return simpleColumns
    }
    return simpleColumns.filter((column) => (visibleColumnIds as Set<Key>).has(column.uid))
  }, [simpleColumns, simpleMode, visibleColumnIds])

  const tableColumns = useMemo<ColumnDef<T, TValue>[]>(() => {
    const selectCol: ColumnDef<T, TValue> | null = enableRowSelection ? {
      id: "__select__",
      size: 40,
      enableSorting: false,
      header: () => {
        return (
          <Checkbox
            checked={allCurrentPageSelected ? true : someCurrentPageSelected ? "indeterminate" : false}
            onCheckedChange={toggleCurrentPageSelection}
            aria-label={t("common.table.selectAll", { defaultValue: "Select page" })}
          />
        )
      },
      cell: ({ row }) => {
        const rowId = resolvedGetRowId(row.original, row.index)
        return (
          <Checkbox
            checked={resolvedSelectedKeys.has(rowId)}
            onCheckedChange={(value) => {
              const next = new Set(resolvedSelectedKeys)
              if (value) {
                next.add(rowId)
              } else {
                next.delete(rowId)
              }
              setSelectedKeys(next)
            }}
            aria-label={t("common.table.selectRow", { defaultValue: "Select row" })}
          />
        )
      },
    } : null

    if (!simpleMode) {
      const cols = [...advancedColumns]
      if (selectCol) cols.unshift(selectCol)
      return cols
    }

    const resolved = activeSimpleColumns.map<ColumnDef<T, TValue>>((column) => ({
      id: column.uid,
      accessorFn: (row) => (row as Record<string, unknown>)[column.uid] as TValue,
      header: column.name,
      enableSorting: column.sortable ?? false,
      size: column.width,
      cell: ({ row }) => {
        if (column.render) {
          return column.render(row.original, column.uid)
        }

        if (column.uid === "actions" && rowActions) {
          return (
            <div className="flex items-center justify-end">
              <Dropdown>
                <DropdownTrigger>
                  <Button isIconOnly size="sm" variant="light" radius="full">
                    <MoreVertical className="h-4 w-4 text-default-400" />
                  </Button>
                </DropdownTrigger>
                <DropdownMenu
                  aria-label={t("common.table.rowActions", {
                    defaultValue: "Row actions",
                  })}
                  onAction={(key) => onRowAction?.(String(key), row.original)}
                >
                  {rowActions.map((action) => (
                    <DropdownItem
                      key={action.key}
                      color={action.color === "danger" ? "danger" : "default"}
                      className={action.color === "danger" ? "text-danger" : ""}
                    >
                      {action.label}
                    </DropdownItem>
                  ))}
                </DropdownMenu>
              </Dropdown>
            </div>
          )
        }

        const value = row.getValue(column.uid)
        return <span>{String(value ?? "")}</span>
      },
    }))

    if (
      rowActions
      && rowActions.length > 0
      && !resolved.some((column) => column.id === "actions")
    ) {
      resolved.push({
        id: "actions",
        header: t("common.actions", { defaultValue: "Actions" }),
        enableSorting: false,
        cell: ({ row }) => (
          <div className="flex items-center justify-end">
            <Dropdown>
              <DropdownTrigger>
                <Button isIconOnly size="sm" variant="light" radius="full">
                  <MoreVertical className="h-4 w-4 text-default-400" />
                </Button>
              </DropdownTrigger>
              <DropdownMenu
                aria-label={t("common.table.rowActions", {
                  defaultValue: "Row actions",
                })}
                onAction={(key) => onRowAction?.(String(key), row.original)}
              >
                {rowActions.map((action) => (
                  <DropdownItem
                    key={action.key}
                    color={action.color === "danger" ? "danger" : "default"}
                    className={action.color === "danger" ? "text-danger" : ""}
                  >
                    {action.label}
                  </DropdownItem>
                ))}
              </DropdownMenu>
            </Dropdown>
          </div>
        ),
      })
    }

    if (selectCol) resolved.unshift(selectCol)
    return resolved
  }, [
    activeSimpleColumns,
    advancedColumns,
    enableRowSelection,
    onRowAction,
    resolvedGetRowId,
    resolvedSelectedKeys,
    rowActions,
    simpleMode,
    t,
  ])

  // TanStack Table exposes non-memo-safe methods. This local disable follows upstream guidance.
  // eslint-disable-next-line react-hooks/incompatible-library
  const table = useReactTable({
    data: filteredData,
    columns: tableColumns,
    state: {
      sorting,
      pagination,
    },
    onSortingChange: setSorting,
    onPaginationChange: setPagination,
    getCoreRowModel: getCoreRowModel(),
    getSortedRowModel: getSortedRowModel(),
    getPaginationRowModel: getPaginationRowModel(),
  })

  const rows = table.getRowModel().rows
  const currentPageRowIds = useMemo(
    () => rows.map((row) => resolvedGetRowId(row.original, row.index)),
    [resolvedGetRowId, rows],
  )
  const allCurrentPageSelected =
    currentPageRowIds.length > 0
    && currentPageRowIds.every((id) => resolvedSelectedKeys.has(id))
  const someCurrentPageSelected =
    !allCurrentPageSelected
    && currentPageRowIds.some((id) => resolvedSelectedKeys.has(id))
  const allFilteredSelected =
    filteredRowIds.length > 0
    && filteredRowIds.every((id) => resolvedSelectedKeys.has(id))
  const canSelectAcrossPages = filteredRowIds.length > currentPageRowIds.length
  const pageCount = Math.max(1, table.getPageCount())
  const totalRows = filteredData.length
  const startRow = totalRows === 0 ? 0 : pagination.pageIndex * pagination.pageSize + 1
  const endRow =
    totalRows === 0 ? 0 : Math.min(totalRows, (pagination.pageIndex + 1) * pagination.pageSize)

  const toggleCurrentPageSelection = (value: boolean) => {
    const next = new Set(resolvedSelectedKeys)
    for (const id of currentPageRowIds) {
      if (value) {
        next.add(id)
      } else {
        next.delete(id)
      }
    }
    setSelectedKeys(next)
  }

  const toggleFilteredSelection = (value: boolean) => {
    const next = new Set(resolvedSelectedKeys)
    for (const id of filteredRowIds) {
      if (value) {
        next.add(id)
      } else {
        next.delete(id)
      }
    }
    setSelectedKeys(next)
  }

  return (
    <div
      className={cn(
        "overflow-hidden rounded-large border-small border-default-200 bg-content1 shadow-small",
        className,
      )}
    >
      {showToolbar ? (
        <div className="flex flex-col gap-4 border-b border-default-200 bg-default-50/50 px-4 py-4 sm:px-5">
          {(title || subtitle) && (
            <div className="space-y-1">
              {title ? (
                <h2 className="text-lg font-semibold tracking-[-0.02em] text-foreground">
                  {title}
                </h2>
              ) : null}
              {subtitle ? (
                <p className="text-sm leading-6 text-default-600">{subtitle}</p>
              ) : null}
            </div>
          )}

          {enableRowSelection && (
            <div className={cn(
              "w-full overflow-hidden transition-all duration-200",
              selectionCount > 0 ? "max-h-14 opacity-100" : "max-h-0 opacity-0 pointer-events-none",
            )}>
              <div className="w-full rounded-large border border-default-200 bg-default-100/80 px-3.5 py-2">
                <div className="flex w-full flex-wrap items-center gap-2.5">
                  {canSelectAcrossPages ? (
                    <>
                      <Checkbox
                        checked={allFilteredSelected}
                        onCheckedChange={toggleFilteredSelection}
                        aria-label={t("common.table.selectFiltered", {
                          defaultValue: "Select all filtered results ({{count}})",
                          count: filteredRowIds.length,
                        })}
                      >
                        {t("common.table.selectFiltered", {
                          defaultValue: "Select all filtered results ({{count}})",
                          count: filteredRowIds.length,
                        })}
                      </Checkbox>
                      <div className="h-4 w-px bg-default-300" />
                    </>
                  ) : null}
                  <span className="text-sm font-semibold tabular-nums text-foreground">
                    {t("common.table.selectedCount", { defaultValue: "{{count}} selected", count: selectionCount })}
                  </span>
                  {batchActions && batchActions.length > 0 && (
                    <>
                      {canSelectAcrossPages ? null : <div className="h-4 w-px bg-default-300" />}
                      {batchActions.map((action) => (
                        <Button
                          key={action.key}
                          size="sm"
                          variant="flat"
                          color={action.color === "danger" ? "danger" : "default"}
                          startContent={action.icon}
                          onPress={() => onBatchAction?.(action.key, [...resolvedSelectedKeys])}
                        >
                          {action.label}
                        </Button>
                      ))}
                    </>
                  )}
                  <div className="ml-auto">
                    <Button
                      isIconOnly
                      size="sm"
                      variant="light"
                      aria-label={t("common.table.clearSelection", { defaultValue: "Clear selection" })}
                      onPress={() => setSelectedKeys(new Set())}
                    >
                      <X className="h-3.5 w-3.5" />
                    </Button>
                  </div>
                </div>
              </div>
            </div>
          )}

          <div className="flex flex-col gap-3 xl:flex-row xl:items-center xl:justify-between">
            <div className="flex min-w-0 flex-wrap items-center gap-2">{filters}</div>
            <div className="flex flex-wrap items-center gap-2 xl:justify-end">
              {actions}

              {statusOptions && statusOptions.length > 0 ? (
                <Dropdown>
                  <DropdownTrigger>
                    <Button
                      color="default"
                      size="sm"
                      endContent={<ChevronDown className="h-4 w-4" />}
                    >
                      {t("common.table.statusLabel", { defaultValue: "Status" })}
                    </Button>
                  </DropdownTrigger>
                  <DropdownMenu
                    aria-label={t("common.table.statusFilter", { defaultValue: "Status filter" })}
                    disallowEmptySelection
                    closeOnSelect={false}
                    selectedKeys={statusFilter}
                    selectionMode="multiple"
                    onSelectionChange={setStatusFilter}
                  >
                    {statusOptions.map((status) => (
                      <DropdownItem key={status.uid}>{capitalizeLabel(status.name)}</DropdownItem>
                    ))}
                  </DropdownMenu>
                </Dropdown>
              ) : null}

              {simpleMode && activeSimpleColumns.length > 0 && simpleColumns.length > 0 ? (
                <Dropdown>
                  <DropdownTrigger>
                    <Button
                      color="default"
                      size="sm"
                      endContent={<ChevronDown className="h-4 w-4" />}
                    >
                      {t("common.table.columns", { defaultValue: "Columns" })}
                    </Button>
                  </DropdownTrigger>
                  <DropdownMenu
                    aria-label={t("common.table.columns", { defaultValue: "Columns" })}
                    disallowEmptySelection
                    closeOnSelect={false}
                    selectionMode="multiple"
                    selectedKeys={visibleColumnIds}
                    onSelectionChange={setVisibleColumnIds}
                  >
                    {simpleColumns
                      .filter((column) => column.uid !== "actions")
                      .map((column) => (
                        <DropdownItem key={column.uid}>
                          {capitalizeLabel(column.name)}
                        </DropdownItem>
                      ))}
                  </DropdownMenu>
                </Dropdown>
              ) : null}

              {primaryActionText ? (
                <Button
                  color="primary"
                  size="sm"
                  endContent={<Plus className="h-4 w-4" />}
                  onPress={onPrimaryAction}
                >
                  {primaryActionText}
                </Button>
              ) : null}

              {enableSearch ? (
                <Input
                  id={searchInputId}
                  aria-label={t("common.table.searchLabel")}
                  isClearable
                  value={keyword}
                  onValueChange={setKeyword}
                  onClear={() => setKeyword("")}
                  placeholder={
                    searchPlaceholder
                    ?? t("common.table.searchPlaceholder", {
                      defaultValue: "Search in current list…",
                    })
                  }
                  startContent={<Search className="h-4 w-4 text-default-400" aria-hidden="true" />}
                  className="w-full sm:w-[280px]"
                  classNames={{
                    inputWrapper:
                      "border-default-200/80 bg-content1/96 shadow-none group-data-[focus=true]:border-primary/50 group-data-[focus=true]:shadow-[0_0_0_3px_hsl(var(--heroui-primary)/0.12)]",
                  }}
                />
              ) : null}
            </div>
          </div>

          {extraTopContent}
        </div>
      ) : null}

      {showPageControls ? (
        <div className="border-b border-default-100 px-4 pb-2 pt-3 sm:px-5">
          <div className="flex flex-wrap items-center justify-between gap-2 text-xs text-default-500">
            <span className="tabular-nums">
              {t("common.table.totalItems", {
                defaultValue: "Total {{count}} items",
                count: totalRows,
              })}
            </span>

            <div className="flex items-center gap-2">
              <span>{t("common.table.rowsPerPage", { defaultValue: "Rows per page" })}</span>
              <Select
                aria-label={t("common.table.rowsPerPage", { defaultValue: "Rows per page" })}
                selectedKeys={new Set([String(pagination.pageSize)])}
                size="sm"
                radius="sm"
                className="w-[106px]"
                classNames={{
                  trigger:
                    "min-h-8 border-default-200/80 bg-content1/96 shadow-none data-[hover=true]:bg-content1",
                  popoverContent: "border border-default-200/80 bg-content1/98",
                  value: "text-sm",
                }}
                onSelectionChange={(selection) => {
                  const nextValue = normalizePageSelection(selection)
                  if (!nextValue) {
                    return
                  }
                  table.setPageSize(Number(nextValue))
                }}
              >
                {normalizedPageSizes.map((size) => (
                  <SelectItem key={String(size)} textValue={String(size)}>
                    {size}
                  </SelectItem>
                ))}
              </Select>
            </div>
          </div>
        </div>
      ) : null}

      <Table
        isHeaderSticky
        aria-label={title ?? t("common.table.dataTableAria", { defaultValue: "Data table" })}
        classNames={{
          base: "min-h-[16rem]",
          wrapper:
            "max-h-[calc(100vh-300px)] overflow-x-auto bg-transparent shadow-none",
          table: cn("w-full min-w-max", density === "compact" && "text-xs"),
          th: cn(
            "bg-default-100/60 text-default-500 text-xs font-semibold uppercase tracking-[0.08em] first:rounded-s-lg last:rounded-e-lg",
            density === "compact" && "text-[10px]",
          ),
          td: cn(
            "py-3 text-sm text-foreground",
            density === "compact" && "py-2 text-xs",
          ),
          tr: "data-[hover=true]:bg-content2/35 transition-colors",
          emptyWrapper: "h-40",
        }}
      >
        <TableHeader>
          {table.getHeaderGroups().flatMap((headerGroup) =>
            headerGroup.headers.map((header) => {
              if (header.isPlaceholder) {
                return <TableColumn key={header.id}>{""}</TableColumn>
              }

              const canSort = header.column.getCanSort()
              const sorted = header.column.getIsSorted()

              return (
                <TableColumn
                  key={header.id}
                  align={header.column.id === "actions" ? "end" : "start"}
                >
                  {canSort ? (
                    <Button
                      type="button"
                      variant="light"
                      size="sm"
                      radius="sm"
                      className="-ml-2 h-8 justify-start px-2 text-xs font-semibold uppercase tracking-[0.12em] text-default-500"
                      onPress={() => header.column.toggleSorting(sorted === "asc")}
                    >
                      <span>
                        {flexRender(header.column.columnDef.header, header.getContext())}
                      </span>
                      {sortIndicator(sorted)}
                    </Button>
                  ) : (
                    flexRender(header.column.columnDef.header, header.getContext())
                  )}
                </TableColumn>
              )
            }),
          )}
        </TableHeader>
        <TableBody
          emptyContent={
            <EmptyState
              icon={<Inbox />}
              title={emptyText ?? t("common.noData", { defaultValue: "No data yet." })}
              size="sm"
            />
          }
          isLoading={isLoading}
          loadingContent={
            <Spinner label={t("common.loading", { defaultValue: "Loading…" })} />
          }
        >
          {rows.map((row) => {
            const resolvedRowClassName =
              typeof rowClassName === "function"
                ? rowClassName(row.original)
                : rowClassName
            const isRowSelected = enableRowSelection && resolvedSelectedKeys.has(resolvedGetRowId(row.original, row.index))

            return (
              <TableRow
                key={row.id}
                className={cn(
                  isRowSelected && "bg-primary/5 data-[hover=true]:bg-primary/8",
                  resolvedRowClassName,
                )}
              >
                {row.getVisibleCells().map((cell) => (
                  <TableCell key={cell.id}>
                    {flexRender(cell.column.columnDef.cell, cell.getContext())}
                  </TableCell>
                ))}
              </TableRow>
            )
          })}
        </TableBody>
      </Table>

      {showPageControls ? (
        <div className="flex flex-col gap-3 border-t border-default-200 bg-default-50/50 px-4 py-3 text-xs text-default-500 sm:px-5 xl:flex-row xl:items-center xl:justify-between">
          <div className="tabular-nums">
            {t("common.table.range", {
              start: startRow,
              end: endRow,
              total: totalRows,
            })}
          </div>

          <div className="flex flex-wrap items-center gap-1.5 xl:justify-end">
            <Button
              type="button"
              variant="light"
              size="sm"
              isIconOnly
              aria-label={t("common.table.firstPage", { defaultValue: "First page" })}
              isDisabled={!table.getCanPreviousPage()}
              onPress={() => table.setPageIndex(0)}
            >
              <ChevronsLeft className="h-4 w-4" />
            </Button>
            <Button
              type="button"
              variant="light"
              size="sm"
              isIconOnly
              aria-label={t("common.table.previousPage", { defaultValue: "Previous page" })}
              isDisabled={!table.getCanPreviousPage()}
              onPress={() => table.previousPage()}
            >
              <ChevronLeft className="h-4 w-4" />
            </Button>

            <Pagination
              isCompact
              showControls={false}
              color="primary"
              page={pagination.pageIndex + 1}
              total={pageCount}
              onChange={(nextPage) => table.setPageIndex(nextPage - 1)}
            />

            <Button
              type="button"
              variant="light"
              size="sm"
              isIconOnly
              aria-label={t("common.table.nextPage", { defaultValue: "Next page" })}
              isDisabled={!table.getCanNextPage()}
              onPress={() => table.nextPage()}
            >
              <ChevronRight className="h-4 w-4" />
            </Button>
            <Button
              type="button"
              variant="light"
              size="sm"
              isIconOnly
              aria-label={t("common.table.lastPage", { defaultValue: "Last page" })}
              isDisabled={!table.getCanNextPage()}
              onPress={() => table.setPageIndex(pageCount - 1)}
            >
              <ChevronsRight className="h-4 w-4" />
            </Button>
          </div>
        </div>
      ) : null}
    </div>
  )
}

// Response types from the FusedFrames API

export interface LibrarySummary {
  id: string;
  name: string;
  description: string | null;
  categories: string[];
  tags: string[];
  documentCount: number;
  createdAt: string;
  updatedAt: string;
}

export interface LibraryDetail extends LibrarySummary {
  edgeCount: number;
}

export interface CategoryCount {
  name: string;
  documentCount: number;
}

export interface TagCount {
  name: string;
  documentCount: number;
}

export interface ApplicationCount {
  name: string;
  documentCount: number;
}

// ─── Document content (template-driven) ─────────────────────────────────────
//
// Every document's body lives in `content`, shaped by the library's document
// template (returned alongside as `schema`). A template is an ordered list of
// sections: a "group" section holds named fields; a "timeline" section holds
// an ordered list of steps, each step holding the named fields. The built-in
// default template is What and why (behaviour, reasoning) · Cause and effect
// (trigger, outcome) · Standard operating procedure (application, instruction,
// detail, expected_result).

export type TemplateFieldType = "text" | "paragraph" | "list";

export interface TemplateField {
  key: string;
  type: TemplateFieldType;
  label: string;
  description: string;
  required: boolean;
  examples: string[];
}

export interface TemplateSection {
  key: string;
  type: "group" | "timeline";
  title: string;
  description?: string;
  fields: TemplateField[];
}

export interface DocumentTemplate {
  version: number;
  sections: TemplateSection[];
}

/** A field value: string for text/paragraph fields, string[] for list fields. */
export type ContentFieldValue = string | string[];

/** Values for one group section, or one timeline step, keyed by field key. */
export type ContentFields = Record<string, ContentFieldValue>;

/**
 * A document's body, keyed by section key: a group section maps to an object
 * of its fields; a timeline section maps to an ordered array of steps.
 */
export type DocumentContent = Record<string, ContentFields | ContentFields[]>;

// ─── Documents ──────────────────────────────────────────────────────────────

export interface DocumentSummary {
  id: string;
  title: string;
  category: string;
  tags: string[];
  applications: string[];
  recordingCount: number;
  deviceCount: number;
  firstSeen: string | null;
  lastSeen: string | null;
  createdAt: string;
  updatedAt: string;
  content: DocumentContent;
  schema: DocumentTemplate;
}

export interface DocumentEdgeOutgoing {
  id: string;
  targetDocumentId: string;
  targetDocumentTitle: string;
  label: string;
  edgeType: "data" | "semantic";
  recordingCount: number;
}

export interface DocumentEdgeIncoming {
  id: string;
  sourceDocumentId: string;
  sourceDocumentTitle: string;
  label: string;
  edgeType: "data" | "semantic";
  recordingCount: number;
}

export interface DocumentEdges {
  outgoing: DocumentEdgeOutgoing[];
  incoming: DocumentEdgeIncoming[];
}

export interface DocumentDetail extends DocumentSummary {
  library: { id: string; name: string };
  edges: DocumentEdges;
}

export interface RecordingQuestion {
  id: string;
  question: string;
  response: string | null;
  sequenceOrder: number;
}

export interface SourceRecording {
  id: string;
  title: string;
  status: string;
  questions: RecordingQuestion[];
  clientCreatedAt: string;
  steps: string[];
}

export interface GraphEdge {
  id: string;
  sourceDocumentId: string;
  targetDocumentId: string;
  label: string;
  edgeType: "data" | "semantic";
  recordingCount: number;
}

export interface TraverseNode {
  id: string;
  title: string;
  category: string;
  applications: string[];
  recordingCount: number;
  depth: number;
  content: DocumentContent;
  schema: DocumentTemplate;
}

// ─── Search ─────────────────────────────────────────────────────────────────

/** Why a document surfaced: the retrieval signals and fused relevance score. */
export interface SearchRelevance {
  signals: string[];
  score: number;
  semanticSimilarity: number | null;
}

export interface SearchDocument {
  id: string;
  title: string;
  category: string;
  tags: string[];
  applications: string[];
  recordingCount: number;
  deviceCount: number;
  firstSeen: string | null;
  lastSeen: string | null;
  content: DocumentContent;
  schema: DocumentTemplate;
  edges: DocumentEdges;
  library: { id: string; name: string };
  relevance: SearchRelevance;
}

export interface LibraryFacet {
  id: string;
  name: string;
  documentCount: number;
}

export interface SearchFacets {
  categories: CategoryCount[];
  tags: TagCount[];
  applications: ApplicationCount[];
  libraries: LibraryFacet[];
}

export interface SearchResult {
  documents: SearchDocument[];
  /** Results after the explicit filters — the count to paginate over. */
  total: number;
  /** Results matching the query before the explicit filters (the facet set). */
  matchedTotal: number;
  page: number;
  pageSize: number;
  facets: SearchFacets;
  /** True when nothing matched and the most active documents were returned. */
  lowConfidence: boolean;
  guidance?: string;
}

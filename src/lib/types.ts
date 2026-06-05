// Response types from the FusedFrames API

export interface LibrarySummary {
  id: string;
  name: string;
  description: string | null;
  categories: string[];
  tags: string[];
  patternCount: number;
  createdAt: string;
  updatedAt: string;
}

export interface LibraryDetail extends LibrarySummary {
  edgeCount: number;
}

export interface CategoryCount {
  name: string;
  patternCount: number;
}

export interface TagCount {
  name: string;
  patternCount: number;
}

export interface ApplicationCount {
  name: string;
  patternCount: number;
}

export interface PatternSummary {
  id: string;
  title: string;
  behaviour: string;
  reasoning: string;
  trigger: string;
  outcome: string;
  category: string;
  tags: string[];
  applications: string[];
  actionCount: number;
  connectionCount: number;
  firstSeen: string | null;
  lastSeen: string | null;
  createdAt: string;
  updatedAt: string;
}

export interface SopStep {
  id: string;
  stepNumber: number;
  application: string;
  instruction: string;
  detail: string | null;
  expectedResult: string;
}

export interface PatternEdgeOutgoing {
  id: string;
  targetPatternId: string;
  targetPatternTitle: string;
  label: string;
  edgeType: "data" | "semantic";
  actionCount: number;
}

export interface PatternEdgeIncoming {
  id: string;
  sourcePatternId: string;
  sourcePatternTitle: string;
  label: string;
  edgeType: "data" | "semantic";
  actionCount: number;
}

export interface PatternDetail extends PatternSummary {
  sopSteps: SopStep[];
  library: { id: string; name: string };
  edges: {
    outgoing: PatternEdgeOutgoing[];
    incoming: PatternEdgeIncoming[];
  };
}

export interface EvidenceQuestion {
  id: string;
  question: string;
  response: string | null;
  sequenceOrder: number;
}

export interface EvidenceAction {
  id: string;
  title: string;
  status: string;
  questions: EvidenceQuestion[];
  clientCreatedAt: string;
  events: string[];
}

export interface GraphEdge {
  id: string;
  sourcePatternId: string;
  targetPatternId: string;
  label: string;
  edgeType: "data" | "semantic";
  actionCount: number;
}

export interface TraverseNode {
  id: string;
  title: string;
  behaviour: string;
  category: string;
  applications: string[];
  actionCount: number;
  depth: number;
}

export interface SearchMatch {
  signals: string[];
  score: number;
  semanticSimilarity: number | null;
}

export interface SearchPattern {
  id: string;
  title: string;
  trigger: string;
  behaviour: string;
  reasoning: string;
  outcome: string;
  category: string;
  tags: string[];
  applications: string[];
  actionCount: number;
  connectionCount: number;
  firstSeen: string | null;
  lastSeen: string | null;
  sopSteps: SopStep[];
  edges: {
    outgoing: PatternEdgeOutgoing[];
    incoming: PatternEdgeIncoming[];
  };
  library: { id: string; name: string };
  relevance: SearchMatch;
}

export interface LibraryFacet {
  id: string;
  name: string;
  patternCount: number;
}

export interface SearchFacets {
  categories: CategoryCount[];
  tags: TagCount[];
  applications: ApplicationCount[];
  libraries: LibraryFacet[];
}

export interface SearchResult {
  patterns: SearchPattern[];
  total: number;
  matchedTotal: number;
  page: number;
  pageSize: number;
  facets: SearchFacets;
  lowConfidence: boolean;
  guidance?: string;
}

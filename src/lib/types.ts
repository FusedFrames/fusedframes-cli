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
  applications: string | null;
  actionCount: number;
  connectionCount: number;
  firstSeen: string | null;
  lastSeen: string | null;
  createdAt: string;
  updatedAt: string;
}

export interface PatternDetail extends PatternSummary {
  library: { id: string; name: string };
  edges: {
    outgoing: { id: string; targetPatternId: string; targetPatternTitle: string; label: string; actionCount: number }[];
    incoming: { id: string; sourcePatternId: string; sourcePatternTitle: string; label: string; actionCount: number }[];
  };
}

export interface EvidenceAction {
  id: string;
  question: string;
  response: string | null;
  createdAt: string;
  events: string[];
}

export interface GraphEdge {
  id: string;
  sourcePatternId: string;
  targetPatternId: string;
  label: string;
  actionCount: number;
}

export interface TraverseNode {
  id: string;
  title: string;
  behaviour: string;
  category: string;
  applications: string | null;
  actionCount: number;
  depth: number;
}

export interface SearchPattern {
  id: string;
  title: string;
  behaviour: string;
  category: string;
  tags: string[];
  applications: string | null;
  actionCount: number;
  connectionCount: number;
  library: { id: string; name: string };
}

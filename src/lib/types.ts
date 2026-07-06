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

export interface DocumentSummary {
  id: string;
  title: string;
  behaviour: string;
  reasoning: string;
  trigger: string;
  outcome: string;
  category: string;
  tags: string[];
  applications: string[];
  recordingCount: number;
  deviceCount: number;
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

export interface DocumentDetail extends DocumentSummary {
  sopSteps: SopStep[];
  library: { id: string; name: string };
  edges: {
    outgoing: DocumentEdgeOutgoing[];
    incoming: DocumentEdgeIncoming[];
  };
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
  behaviour: string;
  category: string;
  applications: string[];
  recordingCount: number;
  depth: number;
}

export interface SearchDocument {
  id: string;
  title: string;
  behaviour: string;
  category: string;
  tags: string[];
  applications: string[];
  recordingCount: number;
  deviceCount: number;
  library: { id: string; name: string };
}

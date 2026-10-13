enum QuestionType {
  fileExists,
  whichCandidate,
  matchReview,
  thinSeeds,
  batchOnly,
  allFailed,
}

class InboxChoice {
  final String key;
  final String label;
  final String? description;
  final bool isDestructive;

  const InboxChoice({
    required this.key,
    required this.label,
    this.description,
    this.isDestructive = false,
  });
}

class InboxItem {
  final int id;
  final int? jobId;
  final int malId;
  final String showTitle;
  final String title;
  final String details;
  final QuestionType type;
  final List<InboxChoice> choices;
  final DateTime createdAt;
  final String? candidateA;
  final String? candidateB;

  const InboxItem({
    required this.id,
    this.jobId,
    required this.malId,
    required this.showTitle,
    required this.title,
    required this.details,
    required this.type,
    required this.choices,
    required this.createdAt,
    this.candidateA,
    this.candidateB,
  });
}

pub const PROBLEM_LIST_QUERY: &str = r#"
query problemsetQuestionList($categorySlug: String, $limit: Int, $skip: Int, $filters: QuestionListFilterInput) {
  problemsetQuestionList: questionList(
    categorySlug: $categorySlug
    limit: $limit
    skip: $skip
    filters: $filters
  ) {
    total: totalNum
    questions: data {
      frontendQuestionId: questionFrontendId
      title
      titleSlug
      difficulty
      status
      acRate
      isPaidOnly
      topicTags {
        name
        slug
      }
    }
  }
}
"#;

pub const QUESTION_DETAIL_QUERY: &str = r#"
query questionDetail($titleSlug: String!) {
  question(titleSlug: $titleSlug) {
    questionId
    frontendQuestionId: questionFrontendId
    title
    titleSlug
    difficulty
    content
    isPaidOnly
    topicTags {
      name
      slug
    }
    codeSnippets {
      lang
      langSlug
      code
    }
    exampleTestcaseList
    sampleTestCase
    hints
    status
  }
}
"#;

pub const GLOBAL_DATA_QUERY: &str = r#"
query {
  userStatus {
    isSignedIn
    username
  }
}
"#;

pub const FAVORITES_LIST_QUERY: &str = r#"
query favoritesList {
  favoritesLists {
    allFavorites {
      idHash
      name
      description
      viewCount
      creator
      isWatched
      isPublicFavorite
      questions {
        questionId
        status
        title
        titleSlug
      }
    }
  }
}
"#;

pub const USER_PROFILE_QUERY: &str = r#"
query getUserProfile($username: String!) {
  matchedUser(username: $username) {
    submitStats {
      acSubmissionNum {
        difficulty
        count
      }
    }
  }
  allQuestionsCount {
    difficulty
    count
  }
}
"#;

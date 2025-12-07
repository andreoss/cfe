Feature: Replies and changes
  A remark may answer another one, and whoever wrote a remark may
  change it or take it away again.

  Scenario: A reply sits under what it answers
    Given the board is answering
    And there is a subject to read
    When an administrator signs in
    And they add a remark to the subject
    And they answer their own remark
    Then the answer is on the subject
    And the answer sits under the remark it answers

  Scenario: A remark may be changed by whoever wrote it
    Given the board is answering
    And there is a subject to read
    When an administrator signs in
    And they add a remark to the subject
    And they change what the remark says
    Then the subject shows the changed words
    And the subject no longer shows the words that were replaced

  Scenario: A remark may be taken away by whoever wrote it
    Given the board is answering
    And there is a subject to read
    When an administrator signs in
    And they add a remark to the subject
    And they take the remark away
    Then the remark is no longer offered on the subject

Feature: Subjects
  A subject can be changed by whoever started it, carries the tags it
  was given, and holds answers to answers.

  Scenario: A subject may be changed by whoever started it
    Given the board is answering
    When an administrator signs in
    And they start a subject
    And they change what the subject says
    Then the subject reads differently now

  Scenario: A subject carries the tags it was given
    Given the board is answering
    When an administrator signs in
    And they start a subject with a tag on it
    Then the subject carries that tag

  Scenario: An answer may itself be answered
    Given the board is answering
    And there is a subject to read
    When an administrator signs in
    And they add a remark to the subject
    And they answer their own remark
    And they answer that answer
    Then all three are on the subject
    And the last one sits under the one before it

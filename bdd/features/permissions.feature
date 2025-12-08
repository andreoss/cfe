Feature: Who may change what
  What somebody wrote belongs to them. Another reader may not change it
  or take it away; somebody who looks after the board may.

  Scenario: A reader may not take away another person's remark
    Given the board is answering
    And there is a subject to read
    When an administrator signs in
    And they add a remark to the subject
    And a reader signs in instead
    And that reader tries to take the remark away
    Then the attempt is refused
    And the remark is still on the subject

  Scenario: A reader may not change another person's remark
    Given the board is answering
    And there is a subject to read
    When an administrator signs in
    And they add a remark to the subject
    And a reader signs in instead
    And that reader tries to change the remark
    Then the attempt is refused
    And the remark is still on the subject

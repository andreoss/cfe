Feature: Finding what was written
  Something written on a board can be found afterwards by a word in it.

  Scenario: A subject can be found by an unusual word in it
    Given the board is answering
    When an administrator signs in
    And they start a subject carrying an unusual word
    Then the subject can be found by that word

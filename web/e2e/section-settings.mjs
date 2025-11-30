import { Builder, By, until } from 'selenium-webdriver'
import chrome from 'selenium-webdriver/chrome.js'
import { promoteViaRoot } from './support.mjs'

const baseUrl = process.env.BASE_URL ?? 'http://127.0.0.1:58081'
const chromedriverPath = process.env.CHROMEDRIVER_PATH

function assert(condition, message) {
  if (!condition) throw new Error(message)
}

async function buildDriver() {
  const options = new chrome.Options()
  options.addArguments('--headless=new', '--no-sandbox', '--disable-gpu')
  const builder = new Builder().forBrowser('chrome').setChromeOptions(options)
  if (chromedriverPath) {
    const service = new chrome.ServiceBuilder(chromedriverPath)
    builder.setChromeService(service)
  }
  return builder.build()
}

async function mainText(driver) {
  let last
  for (let attempt = 0; attempt < 20; attempt += 1) {
    try {
      return (await driver.findElement(By.css('main')).getText()).replace(/\s+/g, ' ')
    } catch (err) {
      last = err
      await new Promise((resolve) => setTimeout(resolve, 250))
    }
  }
  throw last
}

async function clickWhenReady(driver, locator, message) {
  await driver.wait(until.elementLocated(locator), 10000, message)
  let last
  for (let attempt = 0; attempt < 20; attempt += 1) {
    try {
      await driver.findElement(locator).click()
      return
    } catch (err) {
      last = err
      await new Promise((resolve) => setTimeout(resolve, 250))
    }
  }
  throw last
}

async function register(driver, username) {
  await driver.get(`${baseUrl}/register`)
  await driver.wait(until.elementLocated(By.name('username')), 10000)
  await driver.findElement(By.name('username')).sendKeys(username)
  await driver.findElement(By.name('email')).sendKeys(`${username}@example.com`)
  await driver.findElement(By.name('password')).sendKeys('correcthorse')
  await driver.findElement(By.css('button[type="submit"]')).click()
  await driver.wait(until.urlIs(`${baseUrl}/`), 10000)
}

async function fill(driver, name, value) {
  const field = await driver.wait(until.elementLocated(By.name(name)), 10000)
  await field.clear()
  await field.sendKeys(value)
}

async function submit(driver, label) {
  await clickWhenReady(driver, By.xpath(`//button[text()='${label}']`), `the ${label} control`)
  await driver.wait(
    async () => {
      const text = await mainText(driver)
      return text.includes('Saved.') || (await driver.findElements(By.css('[role="alert"]'))).length > 0
    },
    10000,
    `${label} should report an outcome`,
  )
  return mainText(driver)
}

async function run() {
  const mod = await buildDriver()
  const reader = await buildDriver()
  const suffix = Date.now().toString(36)
  const modName = `e2e_sect_m_${suffix}`
  const readerName = `e2e_sect_r_${suffix}`
  const sectionSlug = `sect-${suffix}`
  try {
    await register(mod, modName)
    await promoteViaRoot(modName)
    await register(reader, readerName)

    await reader.get(`${baseUrl}/section-settings`)
    await reader.wait(
      async () => (await mainText(reader)).includes('Only a moderator may configure sections.'),
      10000,
      'a reader should be turned away from the settings page',
    )
    const turnedAway = await mainText(reader)
    assert(
      (await reader.findElements(By.name('section-slug'))).length === 0,
      `a reader must not get the forms, saw: ${turnedAway}`,
    )

    await mod.get(`${baseUrl}/section-settings`)
    await mod.wait(until.elementLocated(By.name('section-slug')), 10000)
    const heading = await mainText(mod)
    assert(heading.includes('Section settings'), `the page should be headed, saw: ${heading}`)

    await fill(mod, 'section-slug', sectionSlug)
    await fill(mod, 'section-title', 'A Seeded Section')
    let text = await submit(mod, 'Add section')
    assert(text.includes('Saved.'), `adding a section should succeed, saw: ${text}`)
    await mod.wait(
      async () => (await mainText(mod)).includes(sectionSlug),
      10000,
      'the new section should be listed',
    )

    await fill(mod, 'section-slug', sectionSlug)
    await fill(mod, 'section-title', 'Taken Twice')
    text = await submit(mod, 'Add section')
    assert(text.includes('slug taken'), `a repeated slug should be refused, saw: ${text}`)

    await fill(mod, 'rename-slug', sectionSlug)
    await fill(mod, 'rename-title', 'A Renamed Section')
    text = await submit(mod, 'Rename section')
    assert(text.includes('Saved.'), `renaming should succeed, saw: ${text}`)
    await mod.get(`${baseUrl}/`)
    await mod.wait(
      until.elementLocated(By.linkText('A Renamed Section')),
      10000,
      'the new name should reach the navigation',
    )

    await mod.get(`${baseUrl}/section-settings`)
    await fill(mod, 'rename-slug', 'no-such-section')
    await fill(mod, 'rename-title', 'Nothing')
    text = await submit(mod, 'Rename section')
    assert(text.includes('section not found'), `renaming a missing one should say so, saw: ${text}`)

    await fill(mod, 'score-slug', sectionSlug)
    const choice = await mod.findElement(By.name('topics-score'))
    await choice.findElement(By.css("option[value='moderators-only']")).click()
    text = await submit(mod, 'Set standing')
    assert(text.includes('Saved.'), `setting the standing should succeed, saw: ${text}`)

    await reader.get(`${baseUrl}/s/${sectionSlug}`)
    await reader.wait(until.elementLocated(By.css('main')), 10000)
    const readerSees = await mainText(reader)
    assert(
      !readerSees.includes('New topic'),
      `a reader should not be offered posting in a moderators-only section, saw: ${readerSees}`,
    )

    await mod.get(`${baseUrl}/section-settings`)
    await fill(mod, 'score-slug', sectionSlug)
    const reopen = await mod.findElement(By.name('topics-score'))
    await reopen.findElement(By.css("option[value='unrestricted']")).click()
    text = await submit(mod, 'Set standing')
    assert(text.includes('Saved.'), `reopening it should succeed, saw: ${text}`)

    await reader.get(`${baseUrl}/s/${sectionSlug}`)
    await reader.wait(
      until.elementLocated(By.xpath("//button[text()='New topic']")),
      10000,
      'reopening the section should let a reader post there again',
    )

    await mod.get(`${baseUrl}/s/${sectionSlug}/groups`)
    await clickWhenReady(mod, By.xpath("//button[text()='New group']"), 'the new group control')
    await fill(mod, 'name', 'Original Group')
    await fill(mod, 'slug', `grp-${suffix}`)
    await clickWhenReady(mod, By.xpath("//button[text()='Create']"), 'the create group control')
    await mod.wait(
      async () => (await mainText(mod)).includes('Original Group'),
      10000,
      'the group should be created before it is renamed',
    )

    await mod.get(`${baseUrl}/section-settings`)
    await fill(mod, 'group-section', sectionSlug)
    await fill(mod, 'group-slug', `grp-${suffix}`)
    await fill(mod, 'group-title', 'Renamed Group')
    text = await submit(mod, 'Rename group')
    assert(text.includes('Saved.'), `renaming a group should succeed, saw: ${text}`)

    await mod.get(`${baseUrl}/s/${sectionSlug}/groups`)
    await mod.wait(
      async () => (await mainText(mod)).includes('Renamed Group'),
      10000,
      'the new group name should show in the section',
    )

    console.log('e2e: section settings flow passed')
  } finally {
    await mod.quit()
    await reader.quit()
  }
}

run().catch((err) => {
  console.error(err)
  process.exit(1)
})

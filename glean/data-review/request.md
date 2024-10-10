
# Request for data collection review form

**All questions are mandatory. You must receive review from a data steward peer on your responses to these questions before shipping new data collection.**

1) What questions will you answer with this data?

2) Why does Mozilla need to answer these questions?  Are there benefits for users? Do we need this information to address product or business requirements? Some example responses:

* Establish baselines or measure changes in product or platform quality or performance.

* Provide information essential for advancing a business objective such as supporting OKRs.

* Determine whether a product or platform change has an effect on user or browser behavior.

3) What alternative methods did you consider to answer these questions? Why were they not sufficient?

4) Can current instrumentation answer these questions?

5) List all proposed measurements and indicate the category of data collection for each measurement, using the [Firefox data collection categories](https://wiki.mozilla.org/Data_Collection) found on the Mozilla wiki.   

**Note that the data steward reviewing your request will characterize your data collection based on the highest (and most sensitive) category.**

<table>
  <tr>
    <td>Measurement Description</td>
    <td>Data Collection Category</td>
    <td>Tracking Bug #</td>
  </tr>
  <tr>
    <td></td>
    <td></td>
    <td></td>
  </tr>
</table>

6) Please provide a link to the documentation for this data collection which describes the ultimate data set in a public, complete, and accurate way.
 * Often the Privacy Notice for your product will link to where the documentation is expected to be.
 * Common examples for Mozilla products/services:
    * If this collection is Telemetry you can state "This collection is documented in its definitions files Histograms.json, Scalars.yaml, and/or Events.yaml and in the Probe Dictionary at https://probes.telemetry.mozilla.org."
    * If this data is collected using the Glean SDK you can state “This collection is documented in the Glean Dictionary at https://dictionary.telemetry.mozilla.org/"
 * In some cases, documentation is included in the project’s repository.

7) How long will this data be collected?  Choose one of the following:

* This is scoped to a time-limited experiment/project until date MM-DD-YYYY.

* I want this data to be collected for 6 months initially (potentially renewable).

* I want to permanently monitor this data. (put someone’s name here)

8) What populations will you measure?

* Which release channels?

* Which countries?

* Which locales?

* Any other filters?  Please describe in detail below.

9) If this data collection is default on, what is the opt-out mechanism for users?

10) Please provide a general description of how you will analyze this data.

11) Where do you intend to share the results of your analysis?

12) Is there a third-party tool (i.e. not Glean or Telemetry) that you are proposing to use for this data collection? If so:

* Are you using that on the Mozilla backend? Or going directly to the third-party?
